#!/usr/bin/env python3
"""The queue waits, and never mistakes *not yet* for either answer.

`merge-queue.py` differs from `mergeable.py` in one respect and it is the one
worth testing hardest: it is asked repeatedly, of a commit it pushed seconds
ago, so an **absence of evidence** — no run, or only runs that skipped
everything — starts out meaning *GitHub has not caught up* and ends up meaning
#153. Getting that backwards either refuses every branch it pushes or merges
one nothing compiled, and the second is the failure this repo calls worse than
a red gate. Only a red run is trusted immediately, because a failure is not
eventually consistent and an absence demonstrably is.

Tested by import, because the states are ones GitHub produces on its own
schedule and no network test can arrange. Nothing here talks to `gh` or `git`.
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
        self.assertIn("nothing has built", " ".join(lines))

    def test_a_listing_that_omits_the_live_run_is_absence_and_not_an_answer(self):
        """Observed on 2026-08-29, on this branch, from this endpoint.

        `actions/runs?head_sha=…` returned the commit's skipped draft run and
        omitted its live one, seconds after `gh run list` had shown both. Read
        literally that poll says nothing built the commit — the same thing a
        draft's run says — and a queue without the grace would have handed back
        a branch three minutes from green.
        """
        state, lines = asked(run(conclusion="skipped"), waited=1.0, jobs={})
        self.assertEqual(state, queue.WAIT)
        self.assertIn("nothing has built", " ".join(lines))

    def test_that_grace_does_not_outlast_the_window(self):
        # Past it, a commit whose every run skipped everything is #153, and
        # `judge`'s wording is the one that says so.
        state, lines = asked(
            run(conclusion="skipped"),
            waited=queue.RUN_APPEARS_SECONDS + 1,
            jobs={},
        )
        self.assertEqual(state, queue.STOP)
        self.assertIn("ran a single job", " ".join(lines))

    def test_a_red_run_still_stops_inside_the_grace(self):
        # Absence is eventually consistent; a failure is not. Waiting one out
        # would sit through the window to say what the red run already said.
        state, lines = asked(
            run(conclusion="failure"), waited=0.0, jobs={1: [job("t", "failure")]}
        )
        self.assertEqual(state, queue.STOP)
        self.assertIn("failure", " ".join(lines))

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

    def test_a_genuine_pass_goes(self):
        state, lines = asked(run(), jobs={1: [job("test", "success")]})
        self.assertEqual(state, queue.GO)
        self.assertIn("passed", lines[0])

    def test_the_verdict_is_mergeables_and_not_a_second_opinion(self):
        # Whatever `judge` refuses, this refuses. Two definitions of "CI
        # passed" would drift silently in the direction that merges.
        spent = queue.RUN_APPEARS_SECONDS + 1
        for runs, jobs in (
            ((run(conclusion="failure"),), {1: [job("t", "failure")]}),
            ((run(),), {1: [job("t", "skipped")]}),
        ):
            with self.subTest(jobs=jobs):
                verdict = asked(*runs, jobs=jobs, waited=spent)
                self.assertEqual(verdict[0], queue.STOP)


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


class Merging(unittest.TestCase):
    """A failed merge call is sorted by what failed (#495)."""

    # What `gh pr merge` printed when it merged #490 and reported otherwise.
    BAD_GATEWAY = (
        'non-200 OK status code: 502 Bad Gateway body: "<html>\\r\\n'
        '<head><title>502 Bad Gateway</title></head>"'
    )

    def test_a_502_is_transport_and_not_a_refusal(self):
        self.assertTrue(queue.transport(self.BAD_GATEWAY))
        for failure in (
            "HTTP 503: Service Unavailable",
            "Post https://api.github.com/graphql: net/http: TLS handshake timeout",
            "read tcp 10.0.0.2:443: read: connection reset by peer",
            "context deadline exceeded",
        ):
            with self.subTest(failure=failure):
                self.assertTrue(queue.transport(failure))

    def test_a_reasoned_no_is_a_refusal_and_gets_no_second_call(self):
        for failure in (
            "HTTP 409: Head branch was modified. Review and try the merge again.",
            "GraphQL: Pull Request is not mergeable (mergePullRequest)",
            "X Pull request #490 is not mergeable: the base branch policy"
            " prohibits the merge.",
            "HTTP 405: Required status check is expected.",
        ):
            with self.subTest(failure=failure):
                self.assertFalse(queue.transport(failure))

    def test_a_status_number_elsewhere_in_the_message_is_not_a_5xx(self):
        # A PR or run number that happens to start with 5 is not a status.
        self.assertFalse(queue.transport("Pull request #502 is not mergeable"))

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
        with mock.patch.object(queue, "look", return_value=READY), \
            mock.patch.object(queue, "git"), \
            mock.patch.object(queue, "advance", return_value=(SHA, [])), \
            mock.patch.object(queue, "wait_for", return_value=(queue.GO, ["CI passed"])), \
            mock.patch.object(queue.subprocess, "run", return_value=failed), \
            mock.patch.object(queue, "merge_record", return_value=record) as asked, \
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
