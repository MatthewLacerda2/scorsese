#!/usr/bin/env python3
"""The merge path refuses a pull request that closes what it may not (#1221).

`planning` is the one rule the user cannot override, so the check is tested
where it decides — `closing.refusal`, and the two verdicts that ask it,
`mergeable.judge` and the queue's `progress` — on plain dictionaries.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from merge_queue import mergeable, verdict  # noqa: E402
from merge_queue.states import STOP  # noqa: E402

closing = mergeable.closing

_spec = importlib.util.spec_from_file_location("human_checks", SCRIPTS / "human-checks.py")
human_checks = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(human_checks)

READY = {
    "isDraft": False,
    "headRefOid": "c23b7c4f0000000000000000000000000000000a",
    "number": 171,
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
}
GREEN = [{"id": 1, "name": "CI", "head_sha": READY["headRefOid"],
          "status": "completed", "conclusion": "success"}]
JOBS = {1: [{"name": "test", "conclusion": "success"}]}


def issue(*labels: str, state: str = "open", **fields: object) -> dict:
    """GitHub's REST answer for an issue carrying these labels."""
    return {"state": state, "labels": [{"name": name} for name in labels], **fields}


class Referenced(unittest.TestCase):
    def test_every_keyword_and_every_link_is_read(self) -> None:
        pull = {
            "body": "Closes #1\nfixes #2, Resolved: #3\nrefs #4 and see #5\nClose #1",
            "closingIssuesReferences": [{"number": 6}],
        }
        self.assertEqual(closing.referenced(pull), [1, 2, 3, 6])

    def test_a_link_inside_markdown_is_not_a_keyword(self) -> None:
        # How Dependabot quotes an upstream changelog: a link, never `#N` bare.
        pull = {"body": "Fixes [#123](https://github.com/x/y/issues/123)"}
        self.assertEqual(closing.referenced(pull), [])


class Refusal(unittest.TestCase):
    def test_an_open_issue_without_a_stage_label_is_fine(self) -> None:
        self.assertEqual(closing.refusal({7: issue("bug", "agent")}), [])
        self.assertEqual(closing.refusal({}), [])

    def test_each_stage_label_refuses_by_issue_and_label(self) -> None:
        for stage in closing.STAGES:
            lines = closing.refusal({957: issue("feature", stage)})
            self.assertEqual(lines[0], f"it closes #957, which is labelled `{stage}`.")
            self.assertIn("Only the user removes the label", lines[-1])

    def test_a_pull_request_or_a_closed_issue_is_the_wrong_number(self) -> None:
        lines = closing.refusal({
            1: issue(pull_request={"url": "…"}),
            2: issue(state="closed"),
        })
        self.assertIn("#1, which is a pull request", lines[0])
        self.assertIn("#2, which is already closed", lines[1])
        self.assertIn("refs #N", lines[-1])


class Verdicts(unittest.TestCase):
    def test_judge_refuses_a_green_pull_request_that_closes_a_planning_issue(self) -> None:
        pull = {**READY, "closes": {957: issue("planning")}}
        ok, lines = mergeable.judge(pull, GREEN, JOBS)
        self.assertFalse(ok)
        self.assertIn("`planning`", lines[0])

    def test_judge_still_passes_once_the_label_is_gone(self) -> None:
        pull = {**READY, "closes": {957: issue("feature")}}
        self.assertTrue(mergeable.judge(pull, GREEN, JOBS)[0])

    def test_the_queue_stops_before_waiting_on_ci(self) -> None:
        pull = {**READY, "closes": {957: issue("human")}}
        state, lines = verdict.progress(pull, [], {}, waited=0)
        self.assertEqual(state, STOP)
        self.assertIn("`human`", lines[0])


class BothStages(unittest.TestCase):
    def test_only_an_issue_with_both_is_listed(self) -> None:
        found = [
            {"number": 957, "title": "Both", **issue("planning", "human")},
            {"number": 958, "title": "One", **issue("planning")},
            {"number": 959, "title": "A PR", "pull_request": {}, **issue("planning", "human")},
        ]
        self.assertEqual(closing.both_stages(found), ["#957 Both"])

    def test_the_report_says_nothing_when_none_has_both(self) -> None:
        self.assertEqual(human_checks.both_stages(lambda *args: []), "")
        listed = human_checks.both_stages(
            lambda *args: [{"number": 957, "title": "Both", **issue("planning", "human")}]
        )
        self.assertIn("- #957 Both", listed)


if __name__ == "__main__":
    unittest.main()
