#!/usr/bin/env python3
"""Render `cargo mutants`' `outcomes.json` as the Markdown an agent reads.

The same argument as `coverage-summary.py`, which this deliberately mirrors: a
signal nobody reads is not a signal. cargo-mutants' own output is a scrolling
log of every mutant it tried, and the two lines that matter — *which mutations
survived* — are somewhere in the middle of it. What comes out here is only the
survivors, with the mutation spelled out, ordered by file so it reads as a
worklist.

A worklist is the thing it has to stay, which is what the collapsing below is
for. `test_workspace = false` runs only the mutated package's own tests, so a
module whose assertions live in another crate has *every* mutant survive by
construction — 125 rows saying one thing. That is one finding, and it is
reported as one sentence; the rows that remain are rows somebody can act on.

`make mutants` runs exactly what CI runs and then feeds the result through
here. By hand, against a sweep of the whole scoped surface:

    cargo mutants
    python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json

Python and not jq for the reason coverage gives: python3 is the one of the two
that is always already there.

There are two callers and one renderer, which is the point. The `mutants` job
in `.github/workflows/ci.yml` takes the report as it comes out above and posts
it as a pull-request comment. `mutation-sweep.yml` asks for `--issue-body`
instead: the same report, wrapped as the body of the sweep's tracking issue and
followed by a catch-rate history that carries over from the body it is handed.
A second renderer for the same data is how the two would drift.

This never exits non-zero over a surviving mutant. Mutation audits quality; it
does not prove correctness, and some survivors are *equivalent mutants* that
are correct to leave alone. See `.cargo/mutants.toml` for that policy and the
`mutants` job in `.github/workflows/ci.yml` for how this gets routed.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# Above this many survivors in one file, the rows stop being a worklist: what a
# reader learns from the fifteenth is what the first already told them, and the
# count is the finding. Small enough that a table which survives this rule is
# read at a glance — which is the point of it existing at all.
COLLAPSE_AT = 8

# Below this many mutants in a file, "nothing was caught here" is not yet
# evidence of anything: one survivor is one survivor, and the structural
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
    rows.sort(key=lambda m: (m["file"], m["span"]["start"]["line"]))
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
    """Files the mutated crate's own tests do not assert on at all.

    Every mutant of the file survived, and there were enough of them for that
    to mean something. The cause is structural far more often than it is
    per-line: `test_workspace = false` (see `.cargo/mutants.toml`) runs only
    the mutated package's tests, so a module tested from another crate lands
    here whatever its assertions are worth.
    """
    return {
        file
        for file, counts in tally.items()
        if counts["noticed"] == 0 and counts["survived"] >= ENOUGH_TO_CONCLUDE
    }


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


ONE_FINDING = "These files are one finding each, and not one per mutation:"

STRUCTURAL = (
    "Where **nothing at all** was caught, read it as one statement about the"
    " module rather than as a list: *no test in the mutated crate asserts on"
    " this code.* That is usually structural — `test_workspace = false` runs"
    " only the mutated package's own tests, so a module whose assertions live"
    " in another crate has every mutant survive by construction. The answer is"
    " one assertion next to the code, or one written reason it belongs"
    " elsewhere — never one piece of work per mutation."
)


def summary_line(file: str, survivors: int, counts: dict[str, int], silent: bool) -> str:
    """One collapsed file, as the sentence that replaces its rows."""
    total = counts["noticed"] + counts["survived"]
    why = "nothing caught" if silent else "too many to read one at a time"
    return f"- **`{file}`** — {survivors} of {total} mutations survived, {why}."


def row(mutant: dict) -> str:
    where = f"{mutant['file']}:{mutant['span']['start']['line']}"
    fn = (mutant.get("function") or {}).get("function_name", "—")
    return f"| `{where}` | `{fn}` | replaced with `{mutant['replacement']}` |"


def section(
    mutants: list[dict],
    tally: dict[str, dict[str, int]],
    title: str,
    note: str,
    *,
    structural: bool = False,
) -> list[str]:
    """One heading: the files that collapse to a sentence, then the rows left.

    `structural` turns on the unasserted-module reading, which belongs to
    survivors and not to timeouts — a mutation that hung the tests proves they
    reach it.
    """
    if not mutants:
        return []
    silent = unasserted(tally) if structural else set()
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
            out += [STRUCTURAL, ""]
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


# ---------------------------------------------------------------------------
# The sweep's issue body
# ---------------------------------------------------------------------------
#
# `--in-diff` audits a line once, on the pull request that wrote it. The
# scheduled sweep audits the rest, one crate at a time, and reports into a
# single issue it rewrites in place — a monthly pile of comments is a pile
# nobody reads, which is the same argument the PR comment's marker makes.
#
# What must *not* be rewritten in place is the number. One catch rate says
# nothing; the question the sweep exists for is whether assertion health is
# getting better or worse, and that needs a predecessor. So the report on top
# is replaced every run and the table underneath only ever gains a row.

SWEEP_MARKER = "<!-- mutation-sweep -->"

# Everything below this line is history and survives the rewrite. It is a
# marker rather than a heading because a heading is something a human might
# reasonably rename, and renaming it would silently start the trend over.
HISTORY_MARKER = "<!-- mutation-sweep:history -->"

HISTORY_HEADING = [
    "## Catch rate over time",
    "",
    "One row per sweep, newest first. A single catch rate is not a finding —"
    " the shape of the column is.",
    "",
    "| Swept | Crate | Viable | Caught | Survived | Catch rate | Run |",
    "| --- | --- | --- | --- | --- | --- | --- |",
]

# Two years and change of weekly rows. The cap exists because an issue body is
# capped too (65 536 characters), and a trend that silently stopped updating
# because the body got too long would be the worst of both.
HISTORY_LIMIT = 120

# A history row, recognised by the date it starts with. Deliberately not "any
# table row": the heading and its separator are rewritten every run, and only
# the data has to survive.
HISTORY_ROW = re.compile(r"^\| \d{4}-\d{2}-\d{2} \|")

CUT_SHORT = (
    "> **This sweep did not finish.** cargo-mutants recorded a start and no"
    " end, so the run was stopped rather than completed — a step timeout, a"
    " cancelled job, or the tool killed under it. What follows describes the"
    " mutants that *did* run and not the crate, and its history row says so."
    " A truncated sweep reporting as a complete one is the outcome this"
    " schedule exists to avoid."
)


def cut_short(data: dict) -> bool:
    """Whether the run was stopped rather than finished.

    cargo-mutants writes `outcomes.json` as it goes and stamps `end_time` only
    on the way out, so a file with a start and no end is a run that was killed.
    Its running totals are real — they are just totals for a fraction of the
    crate. A run that recorded neither time is a fixture or an empty run, and
    claiming *that* was truncated would be inventing a failure.
    """
    return bool(data.get("start_time")) and not data.get("end_time")


def history_row(data: dict, subject: str, when: str, run_url: str) -> str:
    """This run as the line it adds to the trend.

    `viable` matches the headline's arithmetic — mutants that did not compile
    say nothing about the tests and are left out of both halves. A timeout
    counts as noticed for the same reason it does above: the tests can only
    hang on code they run.
    """
    total, unviable = data["total_mutants"], data["unviable"]
    noticed = data["caught"] + data["timeout"]
    viable = total - unviable
    rate = f"{100 * noticed / viable:.0f}%" if viable else "—"
    counted = f"{viable} (cut short)" if cut_short(data) else str(viable)
    run = f"[log]({run_url})" if run_url else "—"
    return (
        f"| {when} | `{subject}` | {counted} | {noticed} |"
        f" {data['missed']} | {rate} | {run} |"
    )


def previous_rows(body: str) -> list[str]:
    """The history already in the issue, oldest rows dropped past the cap."""
    _, _, history = body.partition(HISTORY_MARKER)
    return [line for line in history.splitlines() if HISTORY_ROW.match(line)][
        : HISTORY_LIMIT - 1
    ]


def sweep_body(report: str, rows: list[str]) -> str:
    """The whole issue body: the latest report, then every reading so far."""
    return "\n".join(
        [
            SWEEP_MARKER,
            "",
            "*Written by `.github/workflows/mutation-sweep.yml`, which rewrites"
            " this issue after every run. Edits above the history marker are"
            " overwritten; the table below is never rewritten, only added to.*",
            "",
            report,
            "",
            HISTORY_MARKER,
            "",
            *HISTORY_HEADING,
            *rows,
            "",
        ]
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    """The argv contract, which the PR comment and the sweep share."""
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

    out = headline(data, scope)
    if cut_short(data):
        out += [CUT_SHORT, ""]
    out += section(
        mutant_rows(outcomes, "Timeout"),
        tally,
        "Timed out",
        "These mutations made the tests hang rather than fail. Usually a loop"
        " bound: worth a look, because a test that can hang can hang for real.",
    )
    out += section(
        mutant_rows(outcomes, "MissedMutant"),
        tally,
        "Survivors",
        "Each row is a change that was made to the code with every test still"
        " passing.",
        structural=True,
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
