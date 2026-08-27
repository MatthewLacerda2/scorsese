#!/usr/bin/env python3
"""Sharded runs add up, and a shard that did not report is never rounded away.

The bug this is written against is #399's, one layer on: a mutation job that
covered less than it was asked about used to say nothing at all, and the fix is
worthless if the merge that replaces it quietly reports four shards' worth of
findings as three. So the assertions here are mostly about *absence* — that a
missing shard, an unreadable one, and one killed by its budget each leave the
merged run stamped as unfinished rather than looking smaller than it was.

Exercised as a subprocess on its argv contract, the way the `mutants` job calls
it and for the same reason `test_mutants_summary.py` gives.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutants-merge.py"

START = "2026-08-26T04:00:00.000Z"
LATER = "2026-08-26T04:20:00.000Z"
LATEST = "2026-08-26T04:31:00.000Z"


def run(*argv: str) -> subprocess.CompletedProcess[str]:
    """The script, invoked as the workflow invokes it."""
    return subprocess.run(
        [sys.executable, str(SCRIPT), *argv], capture_output=True, text=True, check=False
    )


def mutant(summary: str = "CaughtMutant") -> dict:
    """One mutant's outcome, in the shape cargo-mutants records."""
    return {
        "summary": summary,
        "scenario": {
            "Mutant": {
                "file": "crates/core/src/keyframe.rs",
                "span": {"start": {"line": 61}},
                "function": {"function_name": "evaluate"},
                "replacement": "0.0",
            }
        },
    }


def shard(caught: int = 1, missed: int = 0, **fields: object) -> dict:
    """One shard's `outcomes.json`, with its own share of the totals."""
    return {
        "total_mutants": caught + missed,
        "caught": caught,
        "missed": missed,
        "timeout": 0,
        "unviable": 0,
        "outcomes": [mutant() for _ in range(caught)]
        + [mutant("MissedMutant") for _ in range(missed)],
        **fields,
    }


def merged(*shards: dict | str, expect: int | None = None) -> dict:
    """Merge these shard documents and return the one that comes out.

    A `str` stands in for a shard whose file cannot be parsed; the literal text
    is written to disk as-is.
    """
    with tempfile.TemporaryDirectory() as tmp:
        paths = []
        for index, doc in enumerate(shards):
            path = Path(tmp) / f"shard-{index}.json"
            path.write_text(doc if isinstance(doc, str) else json.dumps(doc))
            paths.append(str(path))
        expected = len(shards) if expect is None else expect
        result = run("--expect", str(expected), *paths)
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


class TheShardsAddUp(unittest.TestCase):
    """The ordinary path: four runners, one report, and no arithmetic lost."""

    def test_the_totals_are_the_sum_of_the_shards(self) -> None:
        out = merged(shard(caught=3, missed=1), shard(caught=2, missed=2))

        self.assertEqual(out["total_mutants"], 8)
        self.assertEqual(out["caught"], 5)
        self.assertEqual(out["missed"], 3)

    def test_every_shards_outcomes_are_carried_over(self) -> None:
        # The rows of the report come from here. A shard dropped at this point
        # is a survivor nobody is ever shown.
        out = merged(shard(caught=1, missed=2), shard(caught=0, missed=1))

        survivors = [o for o in out["outcomes"] if o["summary"] == "MissedMutant"]
        self.assertEqual(len(survivors), 3)

    def test_a_run_whose_shards_all_finished_has_an_end(self) -> None:
        # `end_time` is what the renderer reads as "this is the whole story",
        # so it is set only when every shard genuinely got there.
        out = merged(
            shard(start_time=START, end_time=LATER),
            shard(start_time=START, end_time=LATEST),
        )

        self.assertEqual(out["start_time"], START)
        self.assertEqual(out["end_time"], LATEST)


class AShardThatDidNotReport(unittest.TestCase):
    """The case #399 is about: less was measured than was asked about."""

    def test_a_shard_short_of_expect_leaves_the_run_unfinished(self) -> None:
        # Two shards were launched and one uploaded nothing. Reporting the
        # other one as a complete run is the failure this whole change exists
        # to stop.
        out = merged(shard(start_time=START, end_time=LATER), expect=2)

        self.assertEqual(out["start_time"], START)
        self.assertNotIn("end_time", out)

    def test_a_shard_stopped_by_its_budget_leaves_the_run_unfinished(self) -> None:
        # cargo-mutants stamps an end only on the way out, so a shard killed at
        # its time budget has a start and no end. One such shard is enough.
        out = merged(
            shard(start_time=START, end_time=LATER),
            shard(start_time=START),
        )

        self.assertNotIn("end_time", out)

    def test_an_unreadable_shard_is_a_missing_shard_and_not_a_crash(self) -> None:
        # A shard killed part-way through writing leaves truncated JSON. Dying
        # on it would turn a partial answer into no answer at all, which is the
        # state being fixed.
        out = merged(shard(caught=2, start_time=START, end_time=LATER), '{"total_mut')

        self.assertEqual(out["caught"], 2)
        self.assertNotIn("end_time", out)

    def test_a_missing_file_is_named_on_stderr(self) -> None:
        # The job log has to say which shard went quiet; the report says how
        # much went unmeasured, and the two together are the whole answer.
        with tempfile.TemporaryDirectory() as tmp:
            result = run("--expect", "2", str(Path(tmp) / "gone.json"))

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("gone.json", result.stderr)
        self.assertIn("0 of 2 shards reported", result.stderr)


class NoShardsAtAll(unittest.TestCase):
    """A diff with no mutable lines runs no shard, and that is not a failure."""

    def test_expecting_none_and_getting_none_is_an_empty_finished_run(self) -> None:
        result = run("--expect", "0")
        out = json.loads(result.stdout)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(out["total_mutants"], 0)
        self.assertEqual(out["outcomes"], [])
        self.assertNotIn("start_time", out)

    def test_the_renderer_reads_that_as_nothing_to_run(self) -> None:
        # The two scripts are only ever used together, so the contract that
        # matters is what the second one says about the first one's output.
        summary = SCRIPT.parent / "mutants-summary.py"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "merged.json"
            path.write_text(run("--expect", "0").stdout)
            report = subprocess.run(
                [sys.executable, str(summary), str(path)],
                capture_output=True,
                text=True,
                check=False,
            )

        self.assertIn("No mutants to run", report.stdout)
        self.assertNotIn("did not finish", report.stdout)


class TheArgvContract(unittest.TestCase):
    def test_expect_is_required_because_a_default_would_be_a_guess(self) -> None:
        # Without it there is no way to tell four shards that all reported from
        # four of which two vanished, which is the one distinction this makes.
        result = run()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--expect", result.stderr)


if __name__ == "__main__":
    unittest.main()
