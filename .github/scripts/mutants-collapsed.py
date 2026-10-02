#!/usr/bin/env python3
"""The sweep's history row for a week the instrument was broken.

`.github/workflows/mutation-sweep.yml` checks the mutated surface against the
floor in `.cargo/mutants.toml` before it sweeps (#388). When the surface has
collapsed — #363 and #365, where `exclude_re` entries matched every mutant and
left six — a sweep would still run, plan a handful of mutants, catch most of
them, and file a plausible catch rate into #341's history column. That column
is read for its shape over time, so one bad row would sit there for good,
indistinguishable from a healthy one.

So a collapsed week does not sweep. The job fails, and this writes the one
honest thing there is to say about that date: a row in the history that says
the surface had collapsed, with the count and the floor, **instead of** a catch
rate. A missing row would read like a sweep that never ran; a row with a number
in it would be a lie. The report above the history marker is left exactly as
it was — this week produced no report, and the job's failure is what says so.

The floor is read from `.cargo/mutants.toml` and nowhere else, through the same
function `mutation-surface.py` uses, and the history is kept by the same
functions `mutants-summary.py` uses, so this cannot drift from either.

    mutants-collapsed.py LIST_FILE --issue-body CURRENT --subject CRATE [--run-url URL]

prints the issue's new body on stdout.
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from datetime import datetime, timezone
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent


def sibling(name: str) -> ModuleType:
    """A script beside this one, loaded by path — its name has a hyphen in it."""
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), HERE / f"{name}.py")
    if spec is None or spec.loader is None:
        raise SystemExit(f"{name}.py is missing from {HERE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


surface = sibling("mutation-surface")
summary = sibling("mutants-summary")

#: What stands in for a report when the issue has none yet to keep.
NO_REPORT = "*No sweep has reported yet.*"


def collapsed_row(count: int, floor: int, subject: str, when: str, run_url: str) -> str:
    """This week as the line it adds to the trend: no numbers, because none are true."""
    run = f"[log]({run_url})" if run_url else "—"
    return (
        f"| {when} | `{subject}` | — | — | — |"
        f" **surface collapsed** ({count} mutants, floor {floor}): instrument"
        f" broken, not swept | {run} |"
    )


def with_row(current: str, row: str) -> str:
    """The issue body with `row` on top of its history and nothing else changed."""
    above, marker, _ = current.partition(summary.HISTORY_MARKER)
    rows = [row, *summary.previous_rows(current)]
    if not marker:
        return summary.sweep_body(NO_REPORT, rows)
    return "\n".join([above + marker, "", *summary.HISTORY_HEADING, *rows, ""])


def parse_args(argv: list[str]) -> argparse.Namespace:
    """The argv contract the workflow and the tests share."""
    parser = argparse.ArgumentParser(
        prog="mutants-collapsed.py",
        description="Add a collapsed-surface row to the mutation sweep's history.",
    )
    parser.add_argument("listing", type=Path, help="the captured `cargo mutants --list`")
    parser.add_argument(
        "--config",
        type=Path,
        default=surface.DEFAULT_CONFIG,
        help="the `.cargo/mutants.toml` holding the `# surface-floor:` line",
    )
    parser.add_argument(
        "--issue-body",
        type=Path,
        required=True,
        metavar="CURRENT",
        help="the tracking issue's current body (an empty or missing file starts one)",
    )
    parser.add_argument("--subject", required=True, help="the crate this week was on")
    parser.add_argument("--run-url", default="", help="the run, linked in the row")
    parser.add_argument(
        "--now",
        default=datetime.now(timezone.utc).strftime("%Y-%m-%d"),
        help="the date recorded in the row; defaults to today, UTC",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    """Print the new body."""
    args = parse_args(argv)
    floor = surface.floor_from(args.config)
    count = surface.counted(args.listing)
    current = args.issue_body.read_text() if args.issue_body.exists() else ""
    row = collapsed_row(count, floor, args.subject, args.now, args.run_url)
    print(with_row(current, row))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
