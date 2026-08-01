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


class Judgement(unittest.TestCase):
    def assert_refused(self, verdict: tuple[bool, list[str]], because: str) -> None:
        ok, lines = verdict
        self.assertFalse(ok, lines)
        self.assertIn(because, " ".join(lines).lower())

    def test_a_draft_is_refused_because_nothing_checked_it(self):
        self.assert_refused(
            mergeable.judge({**READY, "isDraft": True}, run(), []), "draft"
        )

    def test_no_run_at_all_is_refused_and_named_as_the_bug(self):
        # The headline case: not a red check but an absent one.
        self.assert_refused(mergeable.judge(READY, None, []), "no ci run exists")

    def test_a_run_still_in_flight_is_refused(self):
        verdict = mergeable.judge(
            READY, run(status="in_progress", conclusion=None), []
        )
        self.assert_refused(verdict, "in_progress")

    def test_a_run_that_skipped_everything_is_refused(self):
        # What a run against a draft looks like, left behind by the push that
        # preceded `gh pr ready`. GitHub calls this mergeable.
        self.assert_refused(
            mergeable.judge(READY, run(conclusion="skipped"), []), "skipped every job"
        )

    def test_a_failed_run_is_refused_and_points_at_it(self):
        ok, lines = mergeable.judge(READY, run(conclusion="failure"), [])
        self.assertFalse(ok)
        self.assertIn("failure", " ".join(lines))
        self.assertIn("https://example.invalid/run/1", " ".join(lines))

    def test_success_with_every_job_skipped_is_still_refused(self):
        # Belt and braces against a future partial-skip: the run says success,
        # and success over nothing is not a pass.
        verdict = mergeable.judge(
            READY, run(), [job("test", "skipped"), job("app", "skipped")]
        )
        self.assert_refused(verdict, "no job in it ran")

    def test_a_genuine_pass_is_allowed(self):
        ok, lines = mergeable.judge(READY, run(), [job("test", "success")])
        self.assertTrue(ok, lines)
        self.assertIn("passed", lines[0])

    def test_a_pass_names_the_jobs_that_skipped_by_design(self):
        # The app gate skips when a branch touches nothing under `app/`. Saying
        # so is the difference between this and the report it distrusts.
        ok, lines = mergeable.judge(
            READY, run(), [job("test", "success"), job("app", "skipped")]
        )
        self.assertTrue(ok, lines)
        self.assertIn("app", " ".join(lines[1:]))


class Picking(unittest.TestCase):
    def test_a_run_for_another_commit_is_not_this_commits_run(self):
        stale = run(head_sha="0" * 40)
        self.assertIsNone(mergeable.latest_run([stale], SHA))

    def test_another_workflow_is_not_the_one_that_gates(self):
        other = run(name="coverage")
        self.assertIsNone(mergeable.latest_run([other], SHA))

    def test_the_newest_matching_run_wins(self):
        # A branch drafted, readied, pushed to and readied again has several.
        # GitHub returns them newest-first and the last one is what decides.
        newest, older = run(id=2), run(id=1)
        self.assertEqual(mergeable.latest_run([newest, older], SHA)["id"], 2)


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
