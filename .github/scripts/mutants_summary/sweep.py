"""The sweep's issue body: the latest report over a history it only adds to."""

from __future__ import annotations

import re

from .shortfall import cut_short


# ---------------------------------------------------------------------------
# The sweep's issue body
# ---------------------------------------------------------------------------
#
# A line is audited when somebody asks about it (#651). The
# scheduled sweep audits the rest, one crate at a time, and reports into a
# single issue it rewrites in place — a monthly pile of comments is a pile
# nobody reads.
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
