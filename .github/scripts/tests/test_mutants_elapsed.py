#!/usr/bin/env python3
"""A run's wall-clock time is read from any timestamp cargo-mutants writes, on
any Python this repo runs under.

cargo-mutants writes nanoseconds and a `Z`. Before Python 3.11,
`datetime.fromisoformat` refused both, and a fraction of any length but three
or six digits too, so on macOS's Python 3.9 `make scripts` died on the first
real timestamp a fixture carried. Exercised as a subprocess, on the argv
contract, for the reason `test_mutants_summary.py` gives.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPT = HERE.parent / "mutants-summary.py"
FIXTURE = HERE / "fixtures" / "two-on-one-line.json"


def headline(start: str, end: str) -> str:
    """The report for the two-on-one-line run, timed from `start` to `end`."""
    data = json.loads(FIXTURE.read_text())
    data["start_time"], data["end_time"] = start, end
    with tempfile.TemporaryDirectory() as scratch:
        path = Path(scratch) / "outcomes.json"
        path.write_text(json.dumps(data))
        done = subprocess.run(
            [sys.executable, str(SCRIPT), str(path)],
            capture_output=True,
            text=True,
            check=True,
        )
    return done.stdout


class Elapsed(unittest.TestCase):
    def test_nanoseconds_and_a_z(self) -> None:
        out = headline("2026-09-30T20:08:09.91684074Z", "2026-09-30T20:18:34.472926659Z")
        self.assertIn("10m 25s", out)

    def test_a_fraction_shorter_than_three_digits(self) -> None:
        self.assertIn("30s", headline("2026-09-30T20:08:09Z", "2026-09-30T20:08:39.5Z"))

    def test_an_explicit_offset(self) -> None:
        out = headline("2026-09-30T20:08:09.123+00:00", "2026-09-30T20:09:09.123+00:00")
        self.assertIn("1m 0s", out)


if __name__ == "__main__":
    unittest.main()
