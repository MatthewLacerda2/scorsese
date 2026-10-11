"""Running `mutants-summary.py` the way its callers do, for the tests beside it.

Not a test module (no `test_` prefix, so `unittest discover` leaves it alone):
the fixtures the four `test_mutants_*` files that exercise the renderer share,
split out of `test_mutants_summary.py` with them (#1136). Every helper runs the
script as a subprocess on its argv contract; see that file's docstring for why.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
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


def sweep(previous: str | None, *outcome_list: dict, **counts: int) -> str:
    """The sweep's whole issue body, given the body it is replacing."""
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
        body = Path(tmp) / "current.md"
        if previous is not None:
            body.write_text(previous)
        result = run(
            path,
            "Crate `scorsese-core`.",
            "--issue-body",
            str(body),
            "--subject",
            "scorsese-core",
            "--run-url",
            "https://example.invalid/run/1",
            "--now",
            "2026-08-17",
        )
    assert result.returncode == 0, result.stderr
    return result.stdout


def report(*outcome_list: dict, scope: str = "", args: tuple[str, ...] = (), **counts: int) -> str:
    """The report for this run, rendered with whatever extra argv is passed."""
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
        result = run(path, scope, *args)
    assert result.returncode == 0, result.stderr
    return result.stdout
