#!/usr/bin/env python3
"""Render `cargo mutants`' `outcomes.json` as the Markdown an agent reads.

The same argument as `coverage-summary.py`, which this deliberately mirrors: a
signal nobody reads is not a signal. cargo-mutants' own output is a scrolling
log of every mutant it tried, and the two lines that matter — *which mutations
survived* — are somewhere in the middle of it. What comes out here is only the
survivors, with the mutation spelled out, ordered by file so it reads as a
worklist.

A worklist is the thing it has to stay, which is what the collapsing below is
for. A file the mutated package's own tests never reach has *every* mutant of
it survive — 125 rows saying one thing. That is one finding, and it is reported
as one sentence; the rows that remain are rows somebody can act on. *Why* the
tests never reach it, this deliberately does not guess: see [`NOTHING_CAUGHT`]
and #449 for what guessing cost.

`make mutants` runs the same diff-scoped question an on-request run's `diff`
scope asks, and feeds the result through here. By hand, against a sweep of the whole scoped surface:

    cargo mutants
    python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json

Python and not jq for the reason coverage gives: python3 is the one of the two
that is always already there.

There are several callers and one renderer, which is the point. `make mutants`
and `mutants-on-request.yml` take the report as it comes out above — the
latter as the artifact `make mutants-remote` prints. `mutation-sweep.yml` asks
for `--issue-body` instead: the same report, wrapped as the body of the sweep's tracking issue and
followed by a catch-rate history that carries over from the body it is handed.
A second renderer for the same data is how the two would drift.

Three parts stand on their own and live in the `mutants_summary` package
beside this file (#1136): `rows` renders one survivor, `shortfall` what a run
that did not finish owes its reader, and `sweep` the issue body.

The on-request workflow shards a large scope across several runners, and this
renderer is deliberately unaware of that: `mutants-merge.py` puts the shards
back together first, and what arrives here is the run they add up to. What the
job does hand over is `--in-scope`, the count `cargo mutants --list` gave it before anything was built. That number is what makes an incomplete
report say *how* incomplete: a run stopped by its time budget describes the
mutants it reached, and the difference between the two counts is the gap it is
silent about. Naming that gap is the whole of #399 — a cancelled job reads
exactly like a broken one, and neither reads like the honest answer, which is
*this scope was too large to measure in the time allowed*.

This never exits non-zero over a surviving mutant. Mutation audits quality; it
does not prove correctness, and some survivors are *equivalent mutants* that
are correct to leave alone. See `.cargo/mutants.toml` for that policy and the
`docs/mutation-testing.md` for how a report gets read.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# The package beside this script, importable however the script was loaded —
# `python3` puts this directory on the path, a sibling's `importlib` does not.
HERE = str(Path(__file__).resolve().parent)
if HERE not in sys.path:
    sys.path.insert(0, HERE)

from mutants_summary.rows import row  # noqa: E402
from mutants_summary.shortfall import shortfall  # noqa: E402
# `mutants-collapsed.py` writes the same issue body, through these.
from mutants_summary.sweep import (  # noqa: E402,F401
    HISTORY_HEADING,
    HISTORY_MARKER,
    history_row,
    previous_rows,
    sweep_body,
)

# Above this many survivors in one file, the rows stop being a worklist: what a
# reader learns from the fifteenth is what the first already told them, and the
# count is the finding. Small enough that a table which survives this rule is
# read at a glance — which is the point of it existing at all.
COLLAPSE_AT = 8

# Below this many mutants in a file, "nothing was caught here" is not yet
# evidence of anything: one survivor is one survivor, and the whole-file
# reading below needs a file the run actually sampled.
ENOUGH_TO_CONCLUDE = 3

# Outcomes that mean a test in the mutated crate noticed. A timeout counts —
# the mutant made the tests hang, which they can only do by running the code.
NOTICED = ("CaughtMutant", "Timeout")


def mutant_rows(outcomes: list[dict], wanted: str) -> list[dict]:
    """Every mutant whose outcome was `wanted`, in source order."""
    rows = []
    for outcome in outcomes:
        scenario = outcome.get("scenario")
        if not isinstance(scenario, dict) or outcome.get("summary") != wanted:
            continue
        rows.append(scenario["Mutant"])
    rows.sort(
        key=lambda m: (
            m["file"],
            m["span"]["start"]["line"],
            m["span"]["start"].get("column", 0),
        )
    )
    return rows


def census(outcomes: list[dict]) -> dict[str, dict[str, int]]:
    """Per file: how many of its mutants a local test noticed, and how many not.

    Only these two are counted. A mutant that did not compile says nothing
    about the tests, so it is not in either total and does not dilute the
    "nothing here was noticed" reading.
    """
    tally: dict[str, dict[str, int]] = {}
    for outcome in outcomes:
        scenario = outcome.get("scenario")
        summary = outcome.get("summary")
        if not isinstance(scenario, dict) or summary not in (*NOTICED, "MissedMutant"):
            continue
        counts = tally.setdefault(scenario["Mutant"]["file"], {"noticed": 0, "survived": 0})
        counts["noticed" if summary in NOTICED else "survived"] += 1
    return tally


def by_file(mutants: list[dict]) -> dict[str, list[dict]]:
    """The same mutants, grouped by the file they changed and still in order."""
    groups: dict[str, list[dict]] = {}
    for mutant in mutants:
        groups.setdefault(mutant["file"], []).append(mutant)
    return groups


def unasserted(tally: dict[str, dict[str, int]]) -> set[str]:
    """Files the mutated package's own tests do not assert on at all.

    Every mutant of the file survived, and there were enough of them for that
    to mean something. That is the whole of what this knows — the membership
    test is a count, and a count cannot tell a module tested from another crate
    from one nothing tests yet or one nothing calls. Naming a cause here is
    what [`NOTHING_CAUGHT`] stopped doing.
    """
    return {
        file
        for file, counts in tally.items()
        if counts["noticed"] == 0 and counts["survived"] >= ENOUGH_TO_CONCLUDE
    }


def instant(stamp: str) -> datetime:
    """A cargo-mutants timestamp, read by any Python this repo meets.

    cargo-mutants writes nanoseconds (`20:08:09.91684074Z`), and
    `datetime.fromisoformat` accepts a `Z`, or a fraction that is not exactly
    three or six digits, only from Python 3.11 on. CI has a newer Python;
    macOS ships 3.9, where `make scripts` failed on the first real timestamp a
    fixture carried. So the fraction is padded or cut to six digits — a
    datetime holds microseconds anyway.
    """
    stamp = stamp.replace("Z", "+00:00")
    stamp = re.sub(r"\.(\d+)", lambda m: "." + m.group(1).ljust(6, "0")[:6], stamp)
    return datetime.fromisoformat(stamp)


def elapsed(data: dict) -> str:
    """Wall-clock time of the run, or the empty string if it did not finish."""
    start, end = data.get("start_time"), data.get("end_time")
    if not (start and end):
        return ""
    seconds = (instant(end) - instant(start)).total_seconds()
    return f"{seconds / 60:.0f}m {seconds % 60:.0f}s" if seconds >= 60 else f"{seconds:.0f}s"


# A run that planned mutations and measured none. Its total reads zero — the
# merge sums what the shards reported, and none did — which the headline once
# read as *nothing to run*, the clean bill of health over an absence this file
# exists to refuse (#992: 171 mutants planned, all tested, two survivors, and
# the report said there was nothing in scope). The planned count is the only
# thing that tells the two apart, so this needs `--in-scope`.
NOTHING_REPORTED = [
    "## Mutation",
    "",
    "### Nothing was measured",
    "",
    "The plan had mutations to run and no shard's results reached this report."
    " This is not a clean run: read the shard jobs' logs for what they found.",
    "",
]


def headline(data: dict, scope: str, in_scope: int | None = None) -> list[str]:
    total = data["total_mutants"]
    missed, caught = data["missed"], data["caught"]
    timeout, unviable = data["timeout"], data["unviable"]
    took = elapsed(data)

    # Always stated, and never omitted when the answer is "nothing": a
    # diff-scoped run that quietly compared against the wrong base would report
    # a clean bill of health forever, and this line is what makes that visible
    # instead of reassuring.
    where = [f"*{scope}*", ""] if scope else []

    if total == 0 and in_scope:
        return [*NOTHING_REPORTED, *where]
    if total == 0:
        return [
            "## Mutation",
            "",
            "### No mutants to run",
            "",
            "Nothing in the changed lines is in the mutated surface"
            " (`.cargo/mutants.toml`'s `examine_globs`).",
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


ONE_FINDING = "These files are one finding each, and not one per mutation:"

# What the report says about a file nothing was caught in — and, since #449,
# what it no longer says. It used to name a likely cause ("usually structural"),
# and that cause was wrong both times it appeared: #442 was a branch's own
# untested `serde` defaults, #444 was a public method with no callers anywhere
# in the workspace. Two for two, two different real causes, neither the one
# named. A wrong diagnosis is worse than none, because CLAUDE.md's three exits
# for a survivor include *exclude with a written reason* — and an exclusion
# carrying a plausible argument is approximately permanent.
#
# So this states the one thing the counts actually establish, lists the causes
# without ranking them, and points at the command that settles it in seconds.
NOTHING_CAUGHT = (
    "Where **nothing at all** was caught, read it as one statement about the"
    " file rather than as a list. What is certain: *nothing in the mutated"
    " package asserts on this code.* Why is not — check before deciding, and"
    " `cargo mutants --list` answers it in seconds because it names the"
    " functions and builds nothing. Three causes, in no particular order: the"
    " code has **no test** yet, and one assertion next to it is the fix; the"
    " code has **no callers** anywhere, and deleting it is the fix — the"
    " mutants go with it; or its **assertions live in another crate**, which"
    " `test_workspace = false` does not run, and one written reason is the fix."
    " Whichever it is, it is one piece of work — never one per mutation — and"
    " an exclusion written before the check is a guess nobody will revisit."
)


def summary_line(file: str, survivors: int, counts: dict[str, int], silent: bool) -> str:
    """One collapsed file, as the sentence that replaces its rows."""
    total = counts["noticed"] + counts["survived"]
    why = "nothing caught" if silent else "too many to read one at a time"
    return f"- **`{file}`** — {survivors} of {total} mutations survived, {why}."


def section(
    mutants: list[dict],
    tally: dict[str, dict[str, int]],
    title: str,
    note: str,
    *,
    nothing_caught: bool = False,
) -> list[str]:
    """One heading: the files that collapse to a sentence, then the rows left.

    `nothing_caught` turns on the whole-file reading, which belongs to
    survivors and not to timeouts — a mutation that hung the tests proves they
    reach it.
    """
    if not mutants:
        return []
    silent = unasserted(tally) if nothing_caught else set()
    summaries, rows = [], []
    for file, group in by_file(mutants).items():
        counts = tally.get(file, {"noticed": 0, "survived": len(group)})
        if file in silent or len(group) > COLLAPSE_AT:
            summaries.append(summary_line(file, len(group), counts, file in silent))
        else:
            rows.extend(group)

    out = [f"### {title}", ""]
    if summaries:
        out += [ONE_FINDING, "", *summaries, ""]
        if silent:
            out += [NOTHING_CAUGHT, ""]
    if rows:
        out += [note, "", "| Where | Function | Mutation |", "| --- | --- | --- |"]
        out += [row(m) for m in rows] + [""]
    return out


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


def parse_args(argv: list[str]) -> argparse.Namespace:
    """The argv contract, which every caller shares."""
    parser = argparse.ArgumentParser(
        prog="mutants-summary.py",
        description="Render a cargo-mutants run as the Markdown an agent reads.",
    )
    parser.add_argument("outcomes", type=Path, help="the run's outcomes.json")
    parser.add_argument(
        "scope",
        nargs="?",
        default="",
        help="one line saying what was mutated, printed above the report",
    )
    parser.add_argument(
        "--in-scope",
        type=int,
        metavar="N",
        help="how many mutations the run was asked about, from"
        " `cargo mutants --list` — which builds nothing, so the"
        " number is known before the run and survives the run being stopped."
        " A report that covers fewer than this says so, and by how many",
    )
    parser.add_argument(
        "--issue-body",
        type=Path,
        metavar="CURRENT",
        help="render the sweep's whole issue body instead of the bare report,"
        " carrying the catch-rate history over from the issue's current body"
        " in this file (an empty or missing file starts the history)",
    )
    parser.add_argument(
        "--subject",
        default="",
        help="what this sweep covered, named in its history row (e.g. a crate)",
    )
    parser.add_argument(
        "--run-url", default="", help="the CI run this reading came from, linked in its row"
    )
    parser.add_argument(
        "--now",
        default=datetime.now(timezone.utc).strftime("%Y-%m-%d"),
        help="the date recorded in the history row; defaults to today, UTC",
    )
    return parser.parse_args(argv)


def main() -> None:
    args = parse_args(sys.argv[1:])
    scope = args.scope
    data = read(args.outcomes)
    outcomes = data.get("outcomes", [])

    tally = census(outcomes)

    out = headline(data, scope, args.in_scope)
    out += shortfall(data, args.in_scope)
    timeouts = mutant_rows(outcomes, "Timeout")
    survivors = mutant_rows(outcomes, "MissedMutant")
    out += section(
        timeouts,
        tally,
        "Timed out",
        "These mutations made the tests hang rather than fail. Usually a loop"
        " bound: worth a look, because a test that can hang can hang for real.",
    )
    out += section(
        survivors,
        tally,
        "Survivors",
        "Each row is a change that was made to the code with every test still"
        " passing.",
        nothing_caught=True,
    )
    report = "\n".join(out + [FOOTER])

    if args.issue_body is None:
        print(report)
        return

    current = args.issue_body.read_text() if args.issue_body.exists() else ""
    row = history_row(data, args.subject or "the scoped surface", args.now, args.run_url)
    print(sweep_body(report, [row, *previous_rows(current)]))


if __name__ == "__main__":
    main()
