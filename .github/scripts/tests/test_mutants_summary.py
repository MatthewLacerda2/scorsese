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


def mutant(summary: str, file: str, line: int, function: str, replacement: str) -> dict:
    """One mutant and what became of it, shaped as cargo-mutants records it."""
    return {
        "summary": summary,
        "scenario": {
            "Mutant": {
                "file": file,
                "span": {"start": {"line": line}},
                "function": {"function_name": function},
                "replacement": replacement,
            }
        },
    }


def survivor(file: str, line: int, function: str, replacement: str) -> dict:
    """A mutant no test objected to."""
    return mutant("MissedMutant", file, line, function, replacement)


def caught(file: str, line: int) -> dict:
    """A mutant of the same file that a local test did object to."""
    return mutant("CaughtMutant", file, line, "evaluate", "0.0")


def render(*outcome_list: dict, **counts: int) -> str:
    """The report for a run made of these outcomes."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "outcomes.json"
        missed = sum(1 for o in outcome_list if o["summary"] == "MissedMutant")
        path.write_text(
            json.dumps(
                outcomes(
                    total_mutants=len(outcome_list),
                    missed=missed,
                    caught=len(outcome_list) - missed,
                    outcomes=list(outcome_list),
                    **counts,
                )
            )
        )
        result = run(path)
    assert result.returncode == 0, result.stderr
    return result.stdout


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


class Collapsing(unittest.TestCase):
    """125 survivors in one module is one finding, and is reported as one."""

    LAYOUT = "crates/compositor/src/text/layout.rs"

    def test_a_module_nothing_was_caught_in_is_a_sentence_not_a_worklist(self) -> None:
        # The shape #290 was filed over: a module whose assertions live in
        # another crate, so `test_workspace = false` leaves every mutant of it
        # surviving. Naming the 125 mutations individually presents a foregone
        # conclusion as work.
        out = render(*(survivor(self.LAYOUT, 20 + i, f"draw_{i}", "()") for i in range(125)))

        self.assertIn("125 of 125 mutations survived, nothing caught", out)
        self.assertIn("no test in the mutated crate asserts on this code", out)
        self.assertIn("test_workspace = false", out)
        self.assertNotIn("draw_7", out)

    def test_a_long_survivor_list_collapses_even_where_something_was_caught(self) -> None:
        # Not structural — the tests do reach this file — but past a certain
        # length the rows stop being read, so the count is what is reported.
        out = render(
            caught(self.LAYOUT, 10),
            *(survivor(self.LAYOUT, 20 + i, f"draw_{i}", "()") for i in range(12)),
        )

        self.assertIn("12 of 13 mutations survived, too many to read", out)
        self.assertNotIn("nothing caught", out)
        self.assertNotIn("draw_7", out)

    def test_a_short_scattered_list_is_still_listed_row_by_row(self) -> None:
        # The report has to keep doing its job in the ordinary case: a few
        # survivors across a few files are the worklist, and each row names the
        # mutation the reader has to judge.
        out = render(
            caught("crates/core/src/keyframe.rs", 40),
            survivor("crates/core/src/keyframe.rs", 61, "evaluate", "0.0"),
            survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="),
        )

        self.assertIn("crates/core/src/keyframe.rs:61", out)
        self.assertIn("crates/core/src/timeline.rs:184", out)
        self.assertIn("`overlaps`", out)
        self.assertNotIn("one finding each", out)

    def test_one_unopposed_survivor_is_not_yet_a_structural_finding(self) -> None:
        # A single mutant surviving is a single mutant surviving. Calling a
        # file unasserted on that evidence would hide the one row that says
        # what actually happened.
        out = render(survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="))

        self.assertIn("crates/core/src/timeline.rs:184", out)
        self.assertNotIn("nothing caught", out)

    def test_nothing_is_paginated_away(self) -> None:
        # What `MAX_ROWS = 25` used to do. Twenty-six survivors spread thinly
        # are twenty-six things to look at, and the last of them is not a
        # footnote.
        files = [f"crates/core/src/m{i}.rs" for i in range(26)]
        out = render(*(survivor(f, 7, "evaluate", "0.0") for f in files))

        self.assertNotIn("more |", out)
        for file in files:
            self.assertIn(f"{file}:7", out)


class TheArgvContract(unittest.TestCase):
    def test_no_arguments_is_a_usage_error_and_not_a_report(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT)], capture_output=True, text=True, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("usage:", result.stderr)


if __name__ == "__main__":
    unittest.main()
