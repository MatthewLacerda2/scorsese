#!/usr/bin/env python3
"""A network blip is retried, an answer is not, and an outage is never a yes.

`mergeable.gh` is how every question `make mergeable` and `make queue` ask of
GitHub reaches it. Before #615 it exited on any failure, so one TLS handshake
timeout killed a queue mid-wait with no summary — three times in one batch,
each looking like a hand-back until somebody read the log. These tests pin the
three cases the issue names: transient failures that then succeed, a transient
failure that never stops, and a 404 that is an answer and must not be asked
twice. Nothing here talks to `gh`: `subprocess.run` and `time.sleep` are
replaced, so the whole backoff costs no wall clock.
"""

from __future__ import annotations

import importlib.util
import subprocess
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


mergeable = load("mergeable", "mergeable.py")
queue = load("merge_queue", "merge-queue.py")

HANDSHAKE = "Get https://api.github.com/repos: net/http: TLS handshake timeout"
NOT_FOUND = "GraphQL: Could not resolve to a PullRequest with the number of 9999."


def done(code: int, stdout: str = "", stderr: str = "") -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess([], code, stdout, stderr)


def ask(*outcomes: subprocess.CompletedProcess):
    """`mergeable.gh` against a fake `gh` answering `outcomes` in turn."""
    with mock.patch.object(mergeable.subprocess, "run", side_effect=outcomes) as run, \
        mock.patch.object(mergeable.time, "sleep") as slept, \
        mock.patch("builtins.print"):
        try:
            return mergeable.gh("pr", "view", "1"), run, slept
        except (mergeable.Unreachable, SystemExit) as stopped:
            return stopped, run, slept


class Retrying(unittest.TestCase):
    def test_a_transient_failure_that_clears_is_an_answer(self):
        answer, run, slept = ask(
            done(1, stderr=HANDSHAKE), done(1, stderr="HTTP 502"), done(0, '{"ok": 1}')
        )
        self.assertEqual(answer, {"ok": 1})
        self.assertEqual(run.call_count, 3)
        self.assertEqual(
            [call.args[0] for call in slept.call_args_list],
            list(mergeable.RETRY_DELAYS[:2]),
        )

    def test_an_outage_that_never_clears_is_unreachable_and_bounded(self):
        tries = len(mergeable.RETRY_DELAYS) + 1
        stopped, run, _ = ask(*[done(1, stderr=HANDSHAKE)] * tries)
        self.assertIsInstance(stopped, mergeable.Unreachable)
        self.assertIn("network", str(stopped))
        self.assertEqual(run.call_count, tries)
        # A few minutes, not forever: the bound is the point.
        self.assertLessEqual(sum(mergeable.RETRY_DELAYS), 300)

    def test_a_404_is_an_answer_and_is_asked_once(self):
        stopped, run, slept = ask(done(1, stderr=NOT_FOUND))
        self.assertIsInstance(stopped, SystemExit)
        self.assertNotIsInstance(stopped, mergeable.Unreachable)
        self.assertEqual(run.call_count, 1)
        slept.assert_not_called()

    def test_a_429_and_a_dns_failure_are_transit(self):
        for failure in (
            "HTTP 429: Too Many Requests",
            "dial tcp: lookup api.github.com: no such host",
            "error connecting to api.github.com",
        ):
            with self.subTest(failure=failure):
                self.assertTrue(mergeable.transport(failure))

    def test_a_4xx_that_means_something_is_not_transit(self):
        for failure in (NOT_FOUND, "HTTP 404: Not Found", "HTTP 422: Validation Failed"):
            with self.subTest(failure=failure):
                self.assertFalse(mergeable.transport(failure))


class Unanswered(unittest.TestCase):
    def test_mergeable_says_network_with_its_own_status_and_never_yes(self):
        outage = mergeable.Unreachable("network: GitHub unreachable")
        with mock.patch.object(mergeable, "answer", side_effect=outage), \
            mock.patch.object(mergeable.sys, "argv", ["mergeable.py", "1"]), \
            mock.patch("builtins.print") as printed:
            status = mergeable.main()
        self.assertEqual(status, mergeable.UNREACHABLE_STATUS)
        self.assertNotIn(status, (0, 1))
        self.assertIn("network", printed.call_args_list[0].args[0])

    def test_the_queue_stops_with_a_summary_and_names_what_it_did_not_reach(self):
        # The queue's own copy of the module, whose class it catches.
        outage = queue.mergeable.Unreachable("network: GitHub unreachable")
        took = [(486, queue.MERGED, "CI passed"), outage]
        with mock.patch.object(queue.mergeable, "gh", return_value={"nameWithOwner": "o/r"}), \
            mock.patch.object(queue, "take", side_effect=took) as take, \
            mock.patch("builtins.print") as printed:
            status = queue.main(["486", "488", "489"])
        self.assertEqual(status, mergeable.UNREACHABLE_STATUS)
        self.assertEqual(take.call_count, 2)
        said = "\n".join(str(call.args[0]) for call in printed.call_args_list if call.args)
        self.assertIn("#486: merged", said)
        self.assertIn("#488: unreachable", said)
        self.assertIn("#489: not reached", said)
        self.assertIn("Run it again with the same list", said)


if __name__ == "__main__":
    unittest.main()
