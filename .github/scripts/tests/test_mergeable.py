#!/usr/bin/env python3
"""The merge check refuses a pull request nothing actually compiled.

The bug it is written against is #153, and the shape of that bug is why these
tests exist at all: the branch was mergeable, `gh pr checks` said every check
had finished, and no machine had built the code. A check that only asked "did
anything go red" would have agreed with GitHub.

`judge` is tested by import because it is pure and because the interesting
cases are states GitHub produces rarely and on its own schedule — a run that
skipped everything is not something a test can arrange over the network. The
argv contract is tested as a subprocess, the way `make` invokes it.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mergeable.py"

_spec = importlib.util.spec_from_file_location("mergeable", SCRIPT)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)

SHA = "c23b7c4f0000000000000000000000000000000a"
READY = {"isDraft": False, "headRefOid": SHA, "number": 171}


def run(**fields: object) -> dict:
    """A completed, successful run for [`SHA`], overridden field by field."""
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


def verdict(*runs: dict, jobs: dict[int, list[dict]] | None = None):
    """[`mergeable.judge`] over these runs, with `jobs` defaulting to one
    successful job per run — because "the run built something" is the ordinary
    case and spelling it out in every test would bury the ones that matter."""
    jobs = jobs if jobs is not None else {
        run["id"]: [job("fmt + clippy + test", "success")] for run in runs
    }
    return mergeable.judge(READY, list(runs), jobs)


class Judgement(unittest.TestCase):
    def assert_refused(self, verdict: tuple[bool, list[str]], because: str) -> None:
        ok, lines = verdict
        self.assertFalse(ok, lines)
        self.assertIn(because, " ".join(lines).lower())

    def test_a_draft_is_refused_because_nothing_checked_it(self):
        self.assert_refused(
            mergeable.judge({**READY, "isDraft": True}, [run()], {}), "draft"
        )

    def test_no_run_at_all_is_refused_and_named_as_the_bug(self):
        # The headline case: not a red check but an absent one.
        self.assert_refused(mergeable.judge(READY, [], {}), "no ci run exists")

    def test_a_run_still_in_flight_is_refused(self):
        self.assert_refused(
            verdict(run(status="in_progress", conclusion=None)), "in_progress"
        )

    def test_a_run_that_skipped_everything_is_refused(self):
        # What a run against a draft looks like, left behind by the push that
        # preceded `gh pr ready`. GitHub calls this mergeable.
        self.assert_refused(
            verdict(run(conclusion="skipped"), jobs={}), "ran a single job"
        )

    def test_a_failed_run_is_refused_and_points_at_it(self):
        ok, lines = verdict(run(conclusion="failure"), jobs={})
        self.assertFalse(ok)
        self.assertIn("failure", " ".join(lines))
        self.assertIn("https://example.invalid/run/1", " ".join(lines))

    def test_success_with_every_job_skipped_is_still_refused(self):
        # Belt and braces against a future partial-skip: the run says success,
        # and success over nothing is not a pass.
        self.assert_refused(
            verdict(run(), jobs={1: [job("test", "skipped"), job("app", "skipped")]}),
            "ran a single job",
        )

    def test_a_genuine_pass_is_allowed(self):
        ok, lines = verdict(run(), jobs={1: [job("test", "success")]})
        self.assertTrue(ok, lines)
        self.assertIn("passed", lines[0])

    def test_a_pass_names_the_jobs_that_skipped_by_design(self):
        # The app gate skips when a branch touches nothing under `app/`. Saying
        # so is the difference between this and the report it distrusts.
        ok, lines = verdict(
            run(), jobs={1: [job("test", "success"), job("app", "skipped")]}
        )
        self.assertTrue(ok, lines)
        self.assertIn("app", " ".join(lines[1:]))

    def test_a_green_run_beside_a_skipped_one_is_a_pass(self):
        """#245, in the direction that cost a wasted commit.

        A draft push and the `ready_for_review` that follows produce two runs
        seconds apart — one skipped, one green. Picking by list position
        refused a pull request that had genuinely passed.
        """
        ok, lines = verdict(
            run(id=1, conclusion="skipped"),
            run(id=2),
            jobs={1: [], 2: [job("fmt + clippy + test", "success")]},
        )
        self.assertTrue(ok, lines)
        self.assertIn("2 runs exist", " ".join(lines))

    def test_a_green_run_beside_a_red_one_is_not_a_pass(self):
        """#245, in the direction that matters.

        Nothing about "whichever the API listed first" prefers a skipped run
        over a failing one, so the same flaw could have called this mergeable —
        which is #153 produced by the tool written to prevent it.
        """
        self.assert_refused(
            verdict(
                run(id=1),
                run(id=2, conclusion="failure"),
                jobs={1: [job("test", "success")], 2: [job("test", "failure")]},
            ),
            "failure",
        )

    def test_the_order_of_a_disagreeing_pair_never_decides(self):
        # The whole bug was an ordering dependency, so the fix is only a fix if
        # both orders answer the same.
        green = run(id=1)
        red = run(id=2, conclusion="failure")
        jobs = {1: [job("test", "success")], 2: [job("test", "failure")]}
        first, _ = mergeable.judge(READY, [green, red], jobs)
        second, _ = mergeable.judge(READY, [red, green], jobs)
        self.assertEqual(first, second)
        self.assertFalse(first)


class Picking(unittest.TestCase):
    def test_a_run_for_another_commit_is_not_this_commits_run(self):
        stale = run(head_sha="0" * 40)
        self.assertEqual(mergeable.runs_for([stale], SHA), [])

    def test_another_workflow_is_not_the_one_that_gates(self):
        other = run(name="coverage")
        self.assertEqual(mergeable.runs_for([other], SHA), [])

    def test_every_matching_run_is_kept(self):
        # A branch drafted, readied, pushed to and readied again has several,
        # and #245 is what came of keeping only one of them.
        newest, older = run(id=2), run(id=1)
        kept = mergeable.runs_for([newest, older], SHA)
        self.assertEqual([it["id"] for it in kept], [2, 1])

    def test_a_run_counts_as_having_built_something_only_if_a_job_passed(self):
        self.assertTrue(mergeable.ran_something(run(), {1: [job("t", "success")]}))
        self.assertFalse(mergeable.ran_something(run(), {1: [job("t", "skipped")]}))
        self.assertFalse(mergeable.ran_something(run(), {}))


class Contract(unittest.TestCase):
    def test_it_asks_for_a_pull_request_number(self):
        done = subprocess.run(
            [sys.executable, str(SCRIPT)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("usage", done.stderr.lower())


if __name__ == "__main__":
    unittest.main()
