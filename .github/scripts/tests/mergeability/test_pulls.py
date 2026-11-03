#!/usr/bin/env python3
"""`pulls.py`: GitHub's REST answers in the shape the merge path reads (#1724).

A cloud container's proxy refuses GraphQL, so the queue and `make mergeable`
ask REST and this adapter maps the answer onto the `gh pr view --json` names
the pure functions read. The payload is a recorded `repos/{repo}/pulls/N`,
trimmed to the fields read.
"""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
RECORDED = json.loads((SCRIPTS / "tests/fixtures/rest-pull.json").read_text(encoding="utf-8"))


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


mergeable = load("mergeable", "mergeable.py")
queue = load("merge_queue", "merge-queue.py")
pulls = mergeable.pulls
SHA = RECORDED["head"]["sha"]


def shaped(**changes) -> dict:
    return pulls.from_rest({**RECORDED, **changes})


class Shape(unittest.TestCase):
    def test_a_recorded_pull_request_maps_to_the_fields_the_checks_read(self):
        self.assertEqual(shaped(), {
            "number": 1723, "state": "OPEN", "isDraft": False,
            "headRefOid": SHA, "headRefName": "1695-billed-counts-locally",
            "baseRefName": "main", "body": "Closes #1695\n\nThe change.\n",
            "author": {"login": "MatthewLacerda2"}, "labels": [{"name": "queue"}],
            "createdAt": "2026-11-02T18:31:07Z", "mergedAt": None,
            "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN",
            "closingIssuesReferences": [{"number": 1695}],
        })

    def test_a_draft(self):
        self.assertTrue(shaped(draft=True)["isDraft"])

    def test_dependabot_is_recognised_by_both_readers(self):
        bot = shaped(user={"login": "dependabot[bot]", "type": "Bot"})
        self.assertTrue(queue.watch.is_bot(bot))
        self.assertTrue(mergeable.reviewed.exempt(bot))

    def test_what_the_body_closes_is_the_keyword_rule_negated_or_not(self):
        body = "Closes #12, fixes: #7\n\nrefs #30, and this does not resolve #9."
        refs = shaped(body=body)["closingIssuesReferences"]
        self.assertEqual(refs, [{"number": 7}, {"number": 9}, {"number": 12}])
        self.assertEqual(shaped(body=None)["closingIssuesReferences"], [])

    def test_every_merge_state_is_the_graphql_word(self):
        for rest in ("clean", "dirty", "unstable", "blocked", "behind", "has_hooks",
                     "draft", "unknown"):
            with self.subTest(rest=rest):
                self.assertEqual(shaped(mergeable_state=rest)["mergeStateStatus"], rest.upper())
        self.assertEqual(shaped(mergeable_state=None)["mergeStateStatus"], "UNKNOWN")

    def test_mergeable_is_a_word_and_not_computed_yet_is_unknown(self):
        for rest, word in ((True, "MERGEABLE"), (False, "CONFLICTING"), (None, "UNKNOWN")):
            with self.subTest(rest=rest):
                self.assertEqual(shaped(mergeable=rest)["mergeable"], word)
        listed = {k: v for k, v in RECORDED.items() if k not in ("mergeable", "mergeable_state")}
        self.assertEqual(pulls.from_rest(listed)["mergeable"], "UNKNOWN")

    def test_state_says_merged_when_rest_says_closed_and_merged(self):
        self.assertEqual(shaped(state="closed")["state"], "CLOSED")
        merged = shaped(state="closed", merged=True, merged_at="2026-11-03T00:00:00Z")
        self.assertEqual(merged["state"], "MERGED")
        self.assertTrue(queue.landed(merged))


class Asking(unittest.TestCase):
    def test_one_pull_request_is_one_rest_call(self):
        gh = mock.Mock(return_value=RECORDED)
        self.assertEqual(pulls.view(gh, "o/r", 1723)["headRefOid"], SHA)
        gh.assert_called_once_with("api", "repos/o/r/pulls/1723")

    def test_the_open_listing_is_read_page_by_page_until_a_short_one(self):
        pages = {1: [RECORDED] * 100, 2: [{**RECORDED, "number": 1}]}
        gh = mock.Mock(side_effect=lambda _api, path: pages[int(path.rsplit("=", 1)[1])])
        listed = pulls.open_pulls(gh, "o/r")
        self.assertEqual(len(listed), 101)
        self.assertEqual(gh.call_args.args,
                         ("api", "repos/o/r/pulls?state=open&per_page=100&page=2"))

    def test_the_label_is_filtered_here(self):
        other = shaped(number=2, labels=[{"name": "bug"}])
        self.assertEqual([p["number"] for p in pulls.labelled([shaped(), other], "queue")], [1723])

    def test_owner_and_name_from_any_remote_spelling(self):
        for url in ("https://github.com/o/r", "https://github.com/o/r.git", "git@github.com:o/r.git",
                    "http://local_proxy@127.0.0.1:1234/git/o/r"):
            with self.subTest(url=url):
                self.assertEqual(pulls.owner_and_name(url), "o/r")


class Writing(unittest.TestCase):
    def test_the_merge_is_a_put_pinned_to_the_head_the_queue_watched(self):
        self.assertEqual(pulls.merge_call("o/r", 7, SHA), [
            "gh", "api", "-X", "PUT", "repos/o/r/pulls/7/merge",
            "-f", "merge_method=squash", "-f", f"sha={SHA}",
        ])

    def test_a_comment_is_a_post_to_the_issue(self):
        self.assertEqual(pulls.comment_call("o/r", 7, "@dependabot rebase"), [
            "gh", "api", "-X", "POST", "repos/o/r/issues/7/comments",
            "-f", "body=@dependabot rebase",
        ])

    def test_a_re_run_of_failed_jobs_is_a_post_to_the_run(self):
        self.assertEqual(pulls.rerun_call("o/r", 42), [
            "gh", "api", "-X", "POST", "repos/o/r/actions/runs/42/rerun-failed-jobs",
        ])


class Mergeable(unittest.TestCase):
    def test_make_mergeable_asks_only_rest(self):
        asked = []

        def gh(*args):
            asked.append(args)
            path = args[1]
            if path == "repos/o/r/pulls/1723":
                return {**RECORDED, "draft": True}
            return [] if "/files?" in path else {}

        with mock.patch.object(mergeable, "gh", side_effect=gh), \
            mock.patch.object(pulls, "repository", return_value="o/r"), \
            mock.patch("builtins.print"):
            self.assertEqual(mergeable.answer("1723"), 1)
        self.assertTrue(asked)
        self.assertTrue(all(args[0] == "api" for args in asked), asked)


if __name__ == "__main__":
    unittest.main()
