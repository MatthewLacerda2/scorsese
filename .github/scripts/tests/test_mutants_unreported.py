#!/usr/bin/env python3
"""A shard's results reach the report wherever the download put them (#992).

Run 38002639511 planned 171 mutants, its one shard tested all of them and found
two survivors, and the report said *No mutants to run*. Two things lined up:
`download-artifact` extracts a lone matching artifact straight into its target
directory rather than into a subdirectory named after it, so the workflow's
`shards/*/outcomes.json` matched nothing; and the renderer read the empty merge
as an empty plan. These pin both ends, through the two scripts' argv contracts,
the way the workflows call them.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]

START = "2026-10-09T23:06:25.000Z"
END = "2026-10-10T00:16:00.000Z"


def script(name: str, *argv: str) -> subprocess.CompletedProcess[str]:
    """One of the scripts, invoked as the workflow invokes it."""
    return subprocess.run(
        [sys.executable, str(SCRIPTS / name), *argv], capture_output=True, text=True, check=False
    )


def outcomes(missed: int, caught: int) -> dict:
    """One finished shard's `outcomes.json`."""

    def mutant(summary: str) -> dict:
        return {
            "summary": summary,
            "scenario": {
                "Mutant": {
                    "file": "crates/zimmer/src/level/integrated.rs",
                    "span": {"start": {"line": 112}},
                    "function": {"function_name": "Integrated::finish"},
                    "replacement": ">=",
                }
            },
        }

    return {
        "total_mutants": missed + caught,
        "missed": missed,
        "caught": caught,
        "timeout": 0,
        "unviable": 0,
        "start_time": START,
        "end_time": END,
        "outcomes": [mutant("MissedMutant")] * missed + [mutant("CaughtMutant")] * caught,
    }


def report(shards: Path, expect: int, in_scope: int) -> str:
    """Merge what is under `shards` and render it, as the report job does."""
    merged = script("mutants-merge.py", "--expect", str(expect), str(shards))
    assert merged.returncode == 0, merged.stderr
    path = shards.parent / "merged.json"
    path.write_text(merged.stdout)
    return script("mutants-summary.py", str(path), "scope", "--in-scope", str(in_scope)).stdout


class EveryLayoutIsRead(unittest.TestCase):
    def test_a_lone_shard_extracted_into_the_directory_itself_is_read(self) -> None:
        # The layout `download-artifact` produced on run 38002639511.
        with tempfile.TemporaryDirectory() as tmp:
            shards = Path(tmp) / "shards"
            shards.mkdir()
            (shards / "outcomes.json").write_text(json.dumps(outcomes(missed=2, caught=168)))
            text = report(shards, expect=1, in_scope=170)

        self.assertIn("2 of 170 mutations survived", text)
        self.assertNotIn("No mutants to run", text)
        self.assertNotIn("not measured", text)

    def test_several_shards_one_per_subdirectory_are_all_read(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            shards = Path(tmp) / "shards"
            for index in range(2):
                (shards / f"mutants-shard-{index}").mkdir(parents=True)
                path = shards / f"mutants-shard-{index}" / "outcomes.json"
                path.write_text(json.dumps(outcomes(missed=1, caught=4)))
            text = report(shards, expect=2, in_scope=10)

        self.assertIn("2 of 10 mutations survived", text)
        self.assertNotIn("not measured", text)


class NothingReportedIsNotNothingPlanned(unittest.TestCase):
    def test_a_plan_with_mutants_and_no_results_says_nothing_was_measured(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            shards = Path(tmp) / "shards"
            shards.mkdir()
            text = report(shards, expect=1, in_scope=171)

        self.assertIn("Nothing was measured", text)
        self.assertNotIn("No mutants to run", text)
        self.assertIn("171 of 171 mutations in scope were not measured", text)

    def test_an_empty_plan_still_reads_as_nothing_to_run(self) -> None:
        # A branch that touched only tests plans nothing and runs no shard.
        with tempfile.TemporaryDirectory() as tmp:
            shards = Path(tmp) / "shards"
            shards.mkdir()
            text = report(shards, expect=0, in_scope=0)

        self.assertIn("No mutants to run", text)
        self.assertNotIn("Nothing was measured", text)


if __name__ == "__main__":
    unittest.main()
