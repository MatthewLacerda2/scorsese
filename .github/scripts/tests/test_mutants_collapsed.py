#!/usr/bin/env python3
"""A collapsed week files one honest row into #341's history, and nothing else.

Written against #388: the sweep would have swept a collapsed surface, caught
most of its handful of mutants and filed a plausible catch rate into a column
that is read for its shape over time. What it writes instead is checked here —
a row that names the collapse and carries no catch rate, on top of a history
that is otherwise untouched, and a report above it that is left alone.

Exercised as the workflow calls it: as a subprocess, on its argv contract.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutants-collapsed.py"

MARKER = "<!-- mutation-sweep:history -->"

OLD_ROW = "| 2026-09-28 | `scorsese-core` | 950 | 900 | 50 | 95% | [log](x) |"

EXISTING = f"""<!-- mutation-sweep -->

*Written by the sweep.*

## Mutation

### 50 of 950 mutations survived

{MARKER}

## Catch rate over time

| Swept | Crate | Viable | Caught | Survived | Catch rate | Run |
| --- | --- | --- | --- | --- | --- | --- |
{OLD_ROW}
"""


class Collapsed(unittest.TestCase):
    """A captured list of six mutants against a floor of 3000."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.dir = Path(scratch.name)
        (self.dir / "list.txt").write_text(
            "\n".join(f"crates/core/src/a.rs:{n}: replace x with y" for n in range(6)) + "\n"
        )
        (self.dir / "mutants.toml").write_text("# surface-floor: 3000\nexamine_globs = []\n")

    def run_with(self, body: str | None, *rest: str) -> subprocess.CompletedProcess[str]:
        current = self.dir / "current.md"
        if body is not None:
            current.write_text(body)
        argv = [
            str(self.dir / "list.txt"), "--config", str(self.dir / "mutants.toml"),
            "--issue-body", str(current), "--subject", "scorsese-zimmer",
            "--now", "2026-10-05", "--run-url", "https://example/run/1", *rest,
        ]
        return subprocess.run(
            [sys.executable, str(SCRIPT), *argv], capture_output=True, text=True, check=False
        )

    def test_the_row_names_the_collapse_and_carries_no_catch_rate(self) -> None:
        done = self.run_with(EXISTING)
        self.assertEqual(done.returncode, 0, done.stderr)
        row = next(line for line in done.stdout.splitlines() if line.startswith("| 2026-10-05"))
        self.assertIn("`scorsese-zimmer`", row)
        self.assertIn("surface collapsed", row)
        self.assertIn("6 mutants, floor 3000", row)
        self.assertIn("[log](https://example/run/1)", row)
        self.assertNotIn("%", row)

    def test_it_goes_on_top_of_the_history_which_is_otherwise_kept(self) -> None:
        lines = self.run_with(EXISTING).stdout.splitlines()
        rows = [line for line in lines if line.startswith("| 2026-")]
        self.assertEqual(len(rows), 2)
        self.assertTrue(rows[0].startswith("| 2026-10-05"))
        self.assertEqual(rows[1], OLD_ROW)

    def test_the_report_above_the_history_is_left_exactly_as_it_was(self) -> None:
        out = self.run_with(EXISTING).stdout
        self.assertEqual(out.partition(MARKER)[0], EXISTING.partition(MARKER)[0])
        self.assertEqual(out.count(MARKER), 1)

    def test_a_row_written_now_survives_the_next_healthy_rewrite(self) -> None:
        """`mutants-summary.py` keeps rows by their leading date; this one has one."""
        out = self.run_with(EXISTING).stdout
        summary = SCRIPT.parent / "mutants-summary.py"
        (self.dir / "outcomes.json").write_text(
            '{"total_mutants": 2, "missed": 0, "caught": 2, "timeout": 0,'
            ' "unviable": 0, "outcomes": []}'
        )
        (self.dir / "after.md").write_text(out)
        nxt = subprocess.run(
            [sys.executable, str(summary), str(self.dir / "outcomes.json"), "",
             "--issue-body", str(self.dir / "after.md"), "--subject", "scorsese-providers",
             "--now", "2026-10-12"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(nxt.returncode, 0, nxt.stderr)
        self.assertIn("surface collapsed", nxt.stdout)
        self.assertIn(OLD_ROW, nxt.stdout)

    def test_an_issue_with_no_history_yet_gets_one(self) -> None:
        done = self.run_with(None)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn(MARKER, done.stdout)
        self.assertIn("surface collapsed", done.stdout)

    def test_a_config_with_no_floor_is_refused(self) -> None:
        """The floor comes from the config only; no line means no row, not a guess."""
        (self.dir / "mutants.toml").write_text("examine_globs = []\n")
        done = self.run_with(EXISTING)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("surface-floor", done.stderr)


if __name__ == "__main__":
    unittest.main()
