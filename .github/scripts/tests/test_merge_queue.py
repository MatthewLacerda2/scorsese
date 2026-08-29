#!/usr/bin/env python3
"""The queue waits, and never mistakes *not yet* for either answer.

`merge-queue.py` differs from `mergeable.py` in one respect and it is the one
worth testing hardest: it is asked repeatedly, of a commit it pushed seconds
ago, so "no run exists" starts out meaning *GitHub has not made one* and ends
up meaning #153. Getting that backwards either refuses every branch it pushes
or merges one nothing compiled, and the second is the failure this repo calls
worse than a red gate.

Tested by import, because the states are ones GitHub produces on its own
schedule and no network test can arrange. Nothing here talks to `gh` or `git`.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
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


def run(**fields: object) -> dict:
    return {
        "name": "CI",
        "head_sha": SHA,
        "id": 1,
        "status": "completed",
        "conclusion": "success",
        "html_url": "https://example.invalid/run/1",
    } | fields


def job(name: str, conclusion: str) -> dict:
    return {"name": name, "conclusion": conclusion}


def asked(*runs: dict, waited: float = 0.0, jobs=None, pull=None):
    """[`queue.progress`] over these runs, one passing job each by default."""
    jobs = jobs if jobs is not None else {
        r["id"]: [job("fmt + clippy + test", "success")] for r in runs
    }
    return queue.progress(pull or READY, list(runs), jobs, waited)


class Polling(unittest.TestCase):
    def test_a_missing_run_is_not_yet_an_answer_right_after_a_push(self):
        # The first poll of every branch this script pushes. Reading it as #153
        # would hand back every single one of them.
        state, lines = asked(waited=1.0)
        self.assertEqual(state, queue.WAIT)
        self.assertIn("not created one", " ".join(lines))

    def test_a_missing_run_becomes_the_answer_once_the_grace_is_spent(self):
        # Past the grace it is #153 or #429, and `mergeable.no_run` is what
        # tells those two apart — so the queue defers to it rather than
        # inventing a third wording.
        state, lines = asked(waited=queue.RUN_APPEARS_SECONDS + 1)
        self.assertEqual(state, queue.STOP)
        self.assertIn("no CI run exists", " ".join(lines))

    def test_a_conflicted_branch_past_the_grace_is_told_it_conflicts(self):
        """#429 reaches the queue through the same door as everywhere else."""
        pull = {**READY, "mergeable": "CONFLICTING", "mergeStateStatus": "DIRTY"}
        state, lines = asked(
            waited=queue.RUN_APPEARS_SECONDS + 1, pull=pull, jobs={}
        )
        self.assertEqual(state, queue.STOP)
        self.assertIn("conflicts with `main`", " ".join(lines))

    def test_a_run_in_flight_is_waited_on(self):
        state, lines = asked(run(status="in_progress", conclusion=None))
        self.assertEqual(state, queue.WAIT)
        self.assertIn("in_progress", " ".join(lines))

    def test_a_failure_stops_even_with_a_live_run_beside_it(self):
        """The order that saves ten minutes, and the one `judge` already uses.

        A commit carrying a red run and a live one is answered. Asking about
        the live one first would sit through the rest of it to be told what the
        red one said at the start.
        """
        state, lines = asked(
            run(id=1, conclusion="failure"),
            run(id=2, status="in_progress", conclusion=None),
            jobs={1: [job("test", "failure")], 2: []},
        )
        self.assertEqual(state, queue.STOP)
        self.assertIn("failure", " ".join(lines))

    def test_a_draft_is_stopped_and_not_waited_on(self):
        state, lines = asked(run(), pull={**READY, "isDraft": True})
        self.assertEqual(state, queue.STOP)
        self.assertIn("draft", " ".join(lines).lower())

    def test_a_run_that_skipped_everything_is_stopped_not_waited_on(self):
        # Success over nothing. Waiting on it would wait forever, because it is
        # already as finished as it is ever going to get.
        state, lines = asked(run(), jobs={1: []})
        self.assertEqual(state, queue.STOP)
        self.assertIn("ran a single job", " ".join(lines))

    def test_a_genuine_pass_goes(self):
        state, lines = asked(run(), jobs={1: [job("test", "success")]})
        self.assertEqual(state, queue.GO)
        self.assertIn("passed", lines[0])

    def test_the_verdict_is_mergeables_and_not_a_second_opinion(self):
        # Whatever `judge` refuses, this refuses. Two definitions of "CI
        # passed" would drift silently in the direction that merges.
        for runs, jobs in (
            ((run(conclusion="failure"),), {1: [job("t", "failure")]}),
            ((run(),), {1: [job("t", "skipped")]}),
        ):
            with self.subTest(jobs=jobs):
                self.assertEqual(asked(*runs, jobs=jobs)[0], queue.STOP)


class Rebasing(unittest.TestCase):
    def test_a_rebase_that_moves_nothing_is_not_pushed(self):
        """The one sound skip #492 asked for.

        Same commit, so the run on record is a run on this exact tree — not an
        equivalent one, the same one.
        """
        self.assertFalse(queue.push_needed(SHA, SHA))
        self.assertTrue(queue.push_needed(SHA, "0" * 40))

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
