"""#1757: the train on GitHub: the rules it needs, its run read, its branches swept, its merges asked again."""

from __future__ import annotations

import subprocess
import unittest
from unittest import mock

from .fakes import merge, queue, rails

GREEN = {"id": 1, "name": "CI", "head_sha": "t", "status": "completed", "conclusion": "success"}
JOBS = {1: [{"name": "fmt + clippy + test", "status": "completed", "conclusion": "success", "steps": [{"status": "completed"}]},
            {"name": "CI ok", "status": "completed", "conclusion": "success", "steps": [{"status": "completed"}]}]}


def rule(strict):
    return [{"type": "pull_request"}, {"type": "required_status_checks",
            "parameters": {"strict_required_status_checks_policy": strict}}]


class Rules(unittest.TestCase):
    def landable(self, answer):
        side = answer if isinstance(answer, BaseException) else None
        with mock.patch.object(queue.mergeable, "gh", return_value=answer, side_effect=side):
            return rails.landable("o/r")

    def test_a_strict_main_keeps_the_watch_one_at_a_time(self):
        self.assertIn("strict_required_status_checks_policy", self.landable(rule(True)))
        self.assertIsNone(self.landable(rule(False)))
        self.assertIsNone(self.landable([{"type": "pull_request"}]))

    def test_rules_github_would_not_show_are_a_reason_too(self):
        self.assertIn("did not show", self.landable(queue.mergeable.Unreachable("down")))
        self.assertIn("did not show", self.landable(queue.mergeable.Answered("403")))

    def test_main_opens_a_yard_only_when_the_rules_allow_it(self):
        opts = queue.parse(["--watch"])
        with mock.patch.object(rails, "landable", return_value="strict"), mock.patch("builtins.print") as said:
            self.assertIsNone(queue.yard.open_yard("o/r", opts))
        self.assertIn("merging one at a time, not in trains of 4: strict.", said.call_args.args[0])
        with mock.patch.object(rails, "landable", return_value=None), \
            mock.patch.object(rails, "sweep", return_value=["merge-train/abc-1-2"]), mock.patch("builtins.print") as said:
            self.assertIsInstance(queue.yard.open_yard("o/r", opts), queue.yard.Yard)
        self.assertIn("watch: merging in trains of up to 4", said.call_args.args[0])


class Reading(unittest.TestCase):
    def test_a_green_run_that_built_the_tip_goes(self):
        self.assertEqual(rails.tip_state([GREEN], JOBS, 60, frozenset())[0], queue.GO)

    def test_a_failed_run_is_red_and_a_live_one_waits(self):
        red = {**GREEN, "conclusion": "failure", "html_url": "u"}
        failed = {1: [{"name": "fmt + clippy + test", "conclusion": "failure", "steps": [{"status": "completed"}]}]}
        self.assertEqual(rails.tip_state([red], failed, 60, frozenset()), (rails.RED, [
            "the train's CI run concluded failure.", "See u."]))
        live = {**GREEN, "status": "in_progress", "conclusion": None}
        self.assertEqual(rails.tip_state([live], {}, 60, frozenset())[0], queue.WAIT)

    def test_no_run_waits_out_its_grace_then_stalls(self):
        self.assertEqual(rails.tip_state([], {}, 60, frozenset())[0], queue.WAIT)
        self.assertEqual(rails.tip_state([], {}, 600, frozenset())[0], rails.STALLED)

    def test_a_run_nobody_judged_is_rerun_once_then_stalls_never_red(self):
        unjudged = {**GREEN, "conclusion": "failure"}
        jobs = {1: [{"name": "fmt + clippy + test", "conclusion": "cancelled", "steps": []}]}
        self.assertEqual(rails.tip_state([unjudged], jobs, 60, frozenset())[0], queue.RERUN)
        self.assertEqual(rails.tip_state([{**unjudged, "run_attempt": 2}], jobs, 60, frozenset())[0], rails.STALLED)


class Housekeeping(unittest.TestCase):
    def test_only_the_queues_own_branches_are_swept(self):
        listing = "a\trefs/heads/merge-train/abc-1-2\nb\trefs/heads/1757-merge-train\n"
        self.assertEqual(rails.left_behind(listing), ["merge-train/abc-1-2"])

    def test_a_base_branch_modified_is_asked_again_then_is_no_fault_of_the_member(self):
        moved = subprocess.CompletedProcess([], 1, "", "HTTP 405: Base branch was modified. Review and try the merge again.")
        done = subprocess.CompletedProcess([], 0, "{}", "")
        with mock.patch.object(merge.subprocess, "run", side_effect=[moved, done]) as run, \
            mock.patch.object(merge.time, "sleep"), mock.patch("builtins.print"):
            self.assertEqual(merge.squash("o/r", 7, "s", 0)[0], queue.MERGED)
        self.assertEqual(run.call_count, 2)
        with mock.patch.object(merge.subprocess, "run", return_value=moved), \
            mock.patch.object(merge.time, "sleep"), mock.patch("builtins.print"):
            self.assertEqual(merge.squash("o/r", 7, "s", 0)[0], merge.BASE_MOVED)


if __name__ == "__main__":
    unittest.main()
