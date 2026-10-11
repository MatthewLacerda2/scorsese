#!/usr/bin/env python3
"""A merge call that failed is sorted by what failed (#495), and the command line.

Split from `test_merge_queue.py` (#1136). [`queue.take`] runs end to end here
with `gh` and `git` replaced; nothing talks to either for real.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
from unittest import mock
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "merge-queue.py"

_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPT)
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)

SHA = "c23b7c4f0000000000000000000000000000000a"
READY = {
    "isDraft": False,
    "headRefOid": SHA,
    "headRefName": "492-merge-queue",
    "number": 492,
    "state": "OPEN",
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
}


class Merging(unittest.TestCase):
    """A failed merge call is sorted by what failed (#495)."""

    # What `gh pr merge` printed when it merged #490 and reported otherwise.
    BAD_GATEWAY = (
        'non-200 OK status code: 502 Bad Gateway body: "<html>\\r\\n'
        '<head><title>502 Bad Gateway</title></head>"'
    )

    def test_a_502_is_transport_and_not_a_refusal(self):
        self.assertTrue(queue.mergeable.transport(self.BAD_GATEWAY))
        for failure in (
            "HTTP 503: Service Unavailable",
            "Post https://api.github.com/graphql: net/http: TLS handshake timeout",
            "read tcp 10.0.0.2:443: read: connection reset by peer",
            "context deadline exceeded",
        ):
            with self.subTest(failure=failure):
                self.assertTrue(queue.mergeable.transport(failure))

    def test_a_reasoned_no_is_a_refusal_and_gets_no_second_call(self):
        for failure in (
            "HTTP 409: Head branch was modified. Review and try the merge again.",
            "GraphQL: Pull Request is not mergeable (mergePullRequest)",
            "X Pull request #490 is not mergeable: the base branch policy"
            " prohibits the merge.",
            "HTTP 405: Required status check is expected.",
        ):
            with self.subTest(failure=failure):
                self.assertFalse(queue.mergeable.transport(failure))

    def test_a_status_number_elsewhere_in_the_message_is_not_a_5xx(self):
        # A PR or run number that happens to start with 5 is not a status.
        self.assertFalse(queue.mergeable.transport("Pull request #502 is not mergeable"))

    def test_the_record_saying_merged_is_a_merge(self):
        for pull in ({"state": "MERGED"}, {"state": "OPEN", "mergedAt": "2026-08-29T01:00:00Z"}):
            with self.subTest(pull=pull):
                state, lines = queue.settled(pull, waited=0.0)
                self.assertEqual(state, queue.GO)
                self.assertIn("it merged", lines[0])

    def test_an_open_record_right_after_the_failure_is_waited_on(self):
        state, _ = queue.settled({"state": "OPEN", "mergedAt": None}, waited=1.0)
        self.assertEqual(state, queue.WAIT)

    def test_github_not_answering_either_is_not_an_answer(self):
        state, _ = queue.settled(None, waited=1.0)
        self.assertEqual(state, queue.WAIT)

    def test_past_the_window_it_is_handed_back_as_unknown_not_refused(self):
        # "Refused" about a merge that may have happened invites a second one.
        spent = queue.MERGE_SETTLES_SECONDS + 1
        for pull in ({"state": "OPEN"}, None):
            with self.subTest(pull=pull):
                state, lines = queue.settled(pull, waited=spent)
                self.assertEqual(state, queue.STOP)
                self.assertIn("unknown", " ".join(lines))
                self.assertNotIn("refused", " ".join(lines))

    def test_ancestry_is_not_how_a_squash_is_recognised(self):
        # `--squash` makes a new commit, so the head is never on `main`; a
        # record that says merged is the whole of the evidence.
        self.assertFalse(queue.landed({"state": "OPEN", "mergedAt": None}))
        self.assertTrue(queue.landed({"state": "MERGED", "mergedAt": None}))


class Taking(unittest.TestCase):
    """[`queue.take`] end to end, with `gh` and `git` replaced (#495)."""

    def take(self, merge_stderr: str, record: dict | None):
        opts = queue.parse(["490", "--poll", "0"])
        failed = subprocess.CompletedProcess([], 1, "", merge_stderr)
        with mock.patch.object(queue.talk, "look", return_value=READY), \
            mock.patch.object(queue.talk, "git"), \
            mock.patch.object(queue.rebase, "advance", return_value=(SHA, [])), \
            mock.patch.object(queue.ci, "wait_for", return_value=(queue.GO, ["CI passed"])), \
            mock.patch.object(queue.subprocess, "run", return_value=failed), \
            mock.patch.object(queue.merge, "merge_record", return_value=record) as asked, \
            mock.patch("builtins.print"):
            return queue.take("o/r", 490, opts), asked

    def test_a_502_that_merged_is_reported_merged(self):
        (_, state, _), _ = self.take(Merging.BAD_GATEWAY, {"state": "MERGED"})
        self.assertEqual(state, queue.MERGED)

    def test_a_refusal_is_handed_back_without_asking_again(self):
        (_, state, why), asked = self.take(
            "GraphQL: Pull Request is not mergeable", {"state": "MERGED"}
        )
        self.assertEqual(state, queue.HANDED_BACK)
        self.assertIn("refused", why)
        asked.assert_not_called()


class Contract(unittest.TestCase):
    def test_it_asks_for_at_least_one_pull_request(self):
        done = subprocess.run(
            [sys.executable, str(SCRIPT)], capture_output=True, text=True, check=False
        )
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("usage", done.stderr.lower())

    def test_every_flag_carries_help_text(self):
        done = subprocess.run(
            [sys.executable, str(SCRIPT), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(done.returncode, 0)
        for flag in ("--no-merge", "--deadline", "--poll", "--root"):
            self.assertIn(flag, done.stdout)
        # An option listed with nothing after it is a flag nobody can use
        # without reading the source, and this repo gates help text on the CLI
        # for exactly that reason.
        for described in ("hand each branch back", "give up waiting", "ask GitHub", "rebase in"):
            self.assertIn(described, done.stdout)

    def test_it_parses_a_queue_of_numbers(self):
        opts = queue.parse(["486", "488", "--no-merge"])
        self.assertEqual(opts.prs, [486, 488])
        self.assertTrue(opts.no_merge)
        self.assertEqual(opts.deadline, queue.DEADLINE_MINUTES)


if __name__ == "__main__":
    unittest.main()
