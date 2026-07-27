#!/usr/bin/env python3
"""The mutation signal renders a run that had nothing to do.

Why this file exists at all: #40's rule that a gate nobody tests can stop
gating without saying so, applied to a signal. The failure it is written
against is a real one — `make mutants` died with a `FileNotFoundError`
traceback, and CI reported "the run did not produce a report", on every branch
that changes only test files. Both said the tool had broken when it had not.

The script is exercised the way its callers call it: as a subprocess, on its
argv contract, asserting on what lands on stdout and on the exit status. A
test that imported the module and called `headline` directly would have passed
throughout the bug, because the bug was never in `headline`.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutants-summary.py"


def run(argument: Path | str, *rest: str) -> subprocess.CompletedProcess[str]:
    """The script, invoked as `make` and the workflow invoke it."""
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(argument), *rest],
        capture_output=True,
        text=True,
        check=False,
    )


def outcomes(**counts: int) -> dict:
    """A run's `outcomes.json`, with the totals a caller wants to state."""
    return {
        "total_mutants": 0,
        "missed": 0,
        "caught": 0,
        "timeout": 0,
        "unviable": 0,
        "outcomes": [],
        **counts,
    }


def survivor(file: str, line: int, function: str, replacement: str) -> dict:
    """One surviving mutant, shaped as cargo-mutants records it."""
    return {
        "summary": "MissedMutant",
        "scenario": {
            "Mutant": {
                "file": file,
                "span": {"start": {"line": line}},
                "function": {"function_name": function},
                "replacement": replacement,
            }
        },
    }


class NothingToRun(unittest.TestCase):
    """The case the script used to die on."""

    def test_a_missing_outcomes_file_is_reported_rather_than_raised(self) -> None:
        # cargo-mutants writes no outcomes.json when the diff it is given holds
        # no mutable lines. A branch that only adds tests is exactly that, and
        # it is the shape of an ordinary pull request — not of a broken tool.
        with tempfile.TemporaryDirectory() as tmp:
            result = run(Path(tmp) / "outcomes.json")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("No mutants to run", result.stdout)
        self.assertNotIn("Traceback", result.stderr)

    def test_nothing_to_run_says_why_rather_than_only_that(self) -> None:
        # The reader has to be able to tell "nothing was in scope" from "the
        # comparison went wrong", which is the whole reason the scope note is
        # never omitted.
        with tempfile.TemporaryDirectory() as tmp:
            result = run(Path(tmp) / "outcomes.json", "compared against `main`.")

        self.assertIn("mutated surface", result.stdout)
        self.assertIn(".cargo/mutants.toml", result.stdout)
        self.assertIn("compared against `main`.", result.stdout)

    def test_an_empty_run_that_did_write_a_file_reads_the_same_way(self) -> None:
        # Whether cargo-mutants wrote a file with no mutants in it or wrote no
        # file at all is an implementation detail of the tool. The report must
        # not depend on which happened.
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(outcomes()))
            written = run(path)
            absent = run(Path(tmp) / "missing.json")

        self.assertEqual(written.stdout, absent.stdout)


class ARunWithResults(unittest.TestCase):
    """The ordinary path, so the fix above cannot swallow a real report."""

    def test_survivors_are_listed_with_the_mutation_spelled_out(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(
                json.dumps(
                    outcomes(
                        total_mutants=2,
                        missed=1,
                        caught=1,
                        outcomes=[survivor("crates/core/src/timeline.rs", 184, "overlaps", "<=")],
                    )
                )
            )
            result = run(path)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("1 of 2 mutations survived", result.stdout)
        self.assertIn("crates/core/src/timeline.rs:184", result.stdout)
        self.assertIn("`overlaps`", result.stdout)
        self.assertIn("<=", result.stdout)

    def test_a_clean_run_says_so_rather_than_saying_nothing_ran(self) -> None:
        # The distinction the bug erased: "every mutation was caught" and
        # "there was nothing to mutate" are different results.
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(outcomes(total_mutants=3, caught=3)))
            result = run(path)

        self.assertIn("All 3 mutations were caught", result.stdout)
        self.assertNotIn("No mutants to run", result.stdout)


class TheArgvContract(unittest.TestCase):
    def test_no_arguments_is_a_usage_error_and_not_a_report(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT)], capture_output=True, text=True, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("usage:", result.stderr)


if __name__ == "__main__":
    unittest.main()
