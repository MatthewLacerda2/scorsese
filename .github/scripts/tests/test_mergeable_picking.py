#!/usr/bin/env python3
"""Which runs are this commit's, the predicates `judge` and the queue share, and the argv.

Split from `test_mergeable.py` (#1136), which keeps the verdict itself. The
predicates are tested by import; the argv contract as a subprocess, the way
`make` invokes it.
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

    def test_failed_and_unfinished_are_one_definition_each(self):
        """Named so `merge-queue.py` polls on the same predicates `judge` uses.

        A polling loop has to ask "already answered?" before "still running?",
        which `judge` does not expose as separate questions. Two spellings of
        *failed* would drift, and the drift would be silent in the direction
        that merges.
        """
        red = run(id=1, conclusion="failure")
        live = run(id=2, status="in_progress", conclusion=None)
        skipped = run(id=3, conclusion="skipped")
        self.assertEqual(mergeable.failed_runs([red, live, skipped]), [red])
        self.assertEqual(mergeable.unfinished([red, live, skipped]), [live])

    def test_a_skipped_run_is_not_a_failed_one(self):
        # It is what CI does to a draft and to a Markdown-only change; reading
        # it as a failure would refuse both.
        self.assertEqual(mergeable.failed_runs([run(conclusion="skipped")]), [])

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
