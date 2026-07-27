#!/usr/bin/env python3
"""Render `cargo mutants`' `outcomes.json` as the Markdown an agent reads.

The same argument as `coverage-summary.py`, which this deliberately mirrors: a
signal nobody reads is not a signal. cargo-mutants' own output is a scrolling
log of every mutant it tried, and the two lines that matter — *which mutations
survived* — are somewhere in the middle of it. What comes out here is only the
survivors, with the mutation spelled out, ordered by file so it reads as a
worklist.

`make mutants` runs exactly what CI runs and then feeds the result through
here. By hand, against a sweep of the whole scoped surface:

    cargo mutants
    python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json

Python and not jq for the reason coverage gives: python3 is the one of the two
that is always already there.

This never exits non-zero over a surviving mutant. Mutation audits quality; it
does not prove correctness, and some survivors are *equivalent mutants* that
are correct to leave alone. See `.cargo/mutants.toml` for that policy and the
`mutants` job in `.github/workflows/ci.yml` for how this gets routed.
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path

# How many survivors the table lists before it says "and N more". A worklist
# longer than this is not a worklist; the full detail is in the job's log and
# in the inline annotations on the diff.
MAX_ROWS = 25


def mutant_rows(outcomes: list[dict], wanted: str) -> list[dict]:
    """Every mutant whose outcome was `wanted`, in source order."""
    rows = []
    for outcome in outcomes:
        scenario = outcome.get("scenario")
        if not isinstance(scenario, dict) or outcome.get("summary") != wanted:
            continue
        rows.append(scenario["Mutant"])
    rows.sort(key=lambda m: (m["file"], m["span"]["start"]["line"]))
    return rows


def elapsed(data: dict) -> str:
    """Wall-clock time of the run, or the empty string if it did not finish."""
    start, end = data.get("start_time"), data.get("end_time")
    if not (start and end):
        return ""
    seconds = (
        datetime.fromisoformat(end.replace("Z", "+00:00"))
        - datetime.fromisoformat(start.replace("Z", "+00:00"))
    ).total_seconds()
    return f"{seconds / 60:.0f}m {seconds % 60:.0f}s" if seconds >= 60 else f"{seconds:.0f}s"


def headline(data: dict, scope: str) -> list[str]:
    total = data["total_mutants"]
    missed, caught = data["missed"], data["caught"]
    timeout, unviable = data["timeout"], data["unviable"]
    took = elapsed(data)

    # Always stated, and never omitted when the answer is "nothing": a
    # diff-scoped run that quietly compared against the wrong base would report
    # a clean bill of health forever, and this line is what makes that visible
    # instead of reassuring.
    where = [f"*{scope}*", ""] if scope else []

    if total == 0:
        return [
            "## Mutation",
            "",
            "### No mutants to run",
            "",
            "Nothing in the changed lines is in the mutated surface"
            " (`crates/core`, `crates/compositor`, and the plan and audio"
            " arithmetic of `crates/render` — see `.cargo/mutants.toml`).",
            "",
            *where,
        ]

    viable = total - unviable
    aside = [f"{caught} caught"]
    if timeout:
        aside.append(f"{timeout} timed out")
    if unviable:
        aside.append(f"{unviable} did not compile")
    if took:
        aside.append(f"in {took}")

    verdict = (
        f"### {missed} of {viable} mutations survived"
        if missed
        else f"### All {viable} mutations were caught"
    )
    lede = (
        "A **survivor** is a change to the code that no test objected to."
        " Either an assertion is missing, or the mutation is *equivalent* —"
        " it does not change behaviour, and the right response is an"
        " `exclude_re` entry with a reason, never a weakened test."
        if missed
        else "Every mutation of this code broke at least one test."
    )
    return ["## Mutation", "", verdict, "", " · ".join(aside) + ".", "", *where, lede, ""]


def table(mutants: list[dict], title: str, note: str) -> list[str]:
    if not mutants:
        return []
    rows = [f"### {title}", "", note, "", "| Where | Function | Mutation |", "| --- | --- | --- |"]
    for m in mutants[:MAX_ROWS]:
        where = f"{m['file']}:{m['span']['start']['line']}"
        fn = (m.get("function") or {}).get("function_name", "—")
        rows.append(f"| `{where}` | `{fn}` | replaced with `{m['replacement']}` |")
    if len(mutants) > MAX_ROWS:
        rows.append(f"| … and {len(mutants) - MAX_ROWS} more | | |")
    return rows + [""]


FOOTER = (
    "<sub><b>This is a signal, not a gate.</b> It can never fail a build, and a"
    " surviving mutant is not automatically a defect. Mutation answers one"
    " question well — <i>does anything assert this at all?</i> — and cannot"
    " answer whether an assertion pins down the <i>right</i> behaviour; that"
    " stays manual. Scope, exclusions and the equivalent-mutant policy live in"
    " <code>.cargo/mutants.toml</code>.</sub>"
)


# What a run that had nothing to do leaves behind, expressed as the report it
# would have written. cargo-mutants writes no `outcomes.json` *at all* when the
# diff it was handed contains no mutable lines — it has nothing to record — and
# a branch that changes only test files is exactly that: `--in-diff` is given a
# non-empty diff, because test files are Rust, and finds nothing in the mutated
# surface inside it.
#
# Reading that as "the file is missing, so something broke" is the mistake this
# avoids. Nothing broke; the honest answer is the one `headline` already gives
# for a run whose total is zero, so this routes to it rather than restating it.
NOTHING_TO_RUN = {"total_mutants": 0, "missed": 0, "caught": 0, "timeout": 0, "unviable": 0}


def read(path: Path) -> dict:
    """The run's outcomes, or an empty run if it never wrote any."""
    if not path.exists():
        return dict(NOTHING_TO_RUN)
    return json.loads(path.read_text())


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: mutants-summary.py <outcomes.json> [scope-note]")
    scope = sys.argv[2] if len(sys.argv) > 2 else ""
    data = read(Path(sys.argv[1]))
    outcomes = data.get("outcomes", [])

    out = headline(data, scope)
    out += table(
        mutant_rows(outcomes, "Timeout"),
        "Timed out",
        "These mutations made the tests hang rather than fail. Usually a loop"
        " bound: worth a look, because a test that can hang can hang for real.",
    )
    out += table(
        mutant_rows(outcomes, "MissedMutant"),
        "Survivors",
        "Each row is a change that was made to the code with every test still"
        " passing.",
    )
    print("\n".join(out + [FOOTER]))


if __name__ == "__main__":
    main()
