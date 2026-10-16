#!/usr/bin/env python3
"""Putting a branch on `main` and telling whose head GitHub reports, then the summary.

Split from `test_queue.py` (#1136): the rebase's pure halves — whether
to push, whose head a poll sees, which paths conflicted — and the order and
report a list run keeps. Tested by import; nothing here talks to `gh` or `git`.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "merge-queue.py"

_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPT)
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)

SHA = "c23b7c4f0000000000000000000000000000000a"


class Rebasing(unittest.TestCase):
    def test_a_rebase_that_moves_nothing_is_not_pushed(self):
        """The one sound skip #492 asked for.

        Same commit, so the run on record is a run on this exact tree — not an
        equivalent one, the same one.
        """
        self.assertFalse(queue.push_needed(SHA, SHA))
        self.assertTrue(queue.push_needed(SHA, "0" * 40))

    def test_the_pre_push_head_is_gitHub_lagging_and_not_somebody_else(self):
        """The race that would otherwise hand back every branch this pushes.

        A force-push and GitHub's view of it are not simultaneous, so the first
        poll after one routinely still reports the pre-push head.
        """
        state, lines = queue.head_state(SHA, "1" * 40, SHA, waited=1.0)
        self.assertEqual(state, queue.WAIT)
        self.assertIn("not caught up", " ".join(lines))

    def test_the_pre_push_head_stops_being_an_excuse_once_the_grace_is_spent(self):
        state, _ = queue.head_state(
            SHA, "1" * 40, SHA, waited=queue.RUN_APPEARS_SECONDS + 1
        )
        self.assertEqual(state, queue.STOP)

    def test_a_third_sha_is_somebody_else_and_stops_immediately(self):
        # Never waited out: merging a commit this queue did not watch is the
        # direction that must not be given the benefit of the doubt.
        state, lines = queue.head_state("2" * 40, "1" * 40, SHA, waited=0.0)
        self.assertEqual(state, queue.STOP)
        self.assertIn("Somebody else pushed", " ".join(lines))

    def test_the_pushed_head_goes(self):
        self.assertEqual(queue.head_state(SHA, SHA, "0" * 40, 0.0)[0], queue.GO)

    def test_a_rebase_that_pushed_nothing_still_reads_as_the_right_head(self):
        # `push_needed` was False, so pushed and before are the same commit.
        self.assertEqual(queue.head_state(SHA, SHA, SHA, 0.0)[0], queue.GO)

    def test_conflicted_paths_are_named_one_per_line(self):
        named = queue.conflicts("crates/zimmer/src/lib.rs\ncrates/zimmer/src/fx.rs\n")
        self.assertEqual(named, ["crates/zimmer/src/lib.rs", "crates/zimmer/src/fx.rs"])

    def test_a_rebase_that_named_no_path_still_reads_as_a_conflict(self):
        # git does not always leave an unmerged entry — an empty list has to
        # mean "nothing to report", never "nothing went wrong".
        self.assertEqual(queue.conflicts("\n  \n"), [])


class Queueing(unittest.TestCase):
    def test_the_order_given_is_the_order_kept(self):
        self.assertEqual(queue.ordered([486, 488, 489]), [486, 488, 489])

    def test_a_number_twice_is_a_typo_and_not_two_merges(self):
        self.assertEqual(queue.ordered([486, 488, 486]), [486, 488])

    def test_a_merge_and_a_green_are_reported_differently(self):
        # `--no-merge` exists, so a queue that called them the same would be
        # claiming a merge it did not make.
        lines = queue.summary(
            [(486, queue.MERGED, "CI passed"), (488, queue.GREEN, "CI passed")]
        )
        self.assertIn("#486: merged", lines[0])
        self.assertIn("#488: green", lines[1])

    def test_every_line_carries_its_reason_including_the_merges(self):
        lines = queue.summary([(486, queue.MERGED, "CI passed on abc1234")])
        self.assertIn("CI passed on abc1234", lines[0])

    def test_merged_branches_are_named_for_cleanup_rather_than_cleaned_up(self):
        # Removing a worktree an agent is standing in is the same class of
        # mistake as rebasing one, so the queue says it instead of doing it.
        lines = queue.summary([(486, queue.MERGED, "CI passed")])
        self.assertIn("#486", lines[-1])
        self.assertIn("worktrees", lines[-1])

    def test_a_hand_back_says_so_without_a_cleanup_note(self):
        lines = queue.summary([(486, queue.HANDED_BACK, "it is a draft")])
        self.assertEqual(len(lines), 1)
        self.assertIn("handed back", lines[0])


if __name__ == "__main__":
    unittest.main()
