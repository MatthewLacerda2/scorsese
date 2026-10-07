#!/usr/bin/env python3
"""`human-checks.py --since T` — every unrun human check merged since T, in one list.

CLAUDE.md makes a check only a human can do (a real window, real speakers,
taste) **never a merge hold**: it goes on the pull request as a checklist for
the user to run later, and the queue moves on. That keeps merging fast, and it
scatters the checks across every description that merged. After the
2026-10-05/06 batch the orchestrator gathered about 25 items out of 14 pull
requests by hand while the maintainer waited for the report (#821) — and
`issue-batch` (*Merging is not shipping*) makes that list the first thing the
report says, so it is a required output, not a convenience.

This prints it as Markdown, ready to paste: a heading per pull request with its
unticked items, then a one-line total. Only two things are read:

- **The human-check sections**, found by their heading at any level and in any
  case. [`HUMAN_HEADING`] holds the spellings merged pull requests actually use
  — *For a human*, *For a human to check (later)*, *Human check(s)*, *Human
  checklist*, *Left for a human*, *For the maintainer*, each sometimes with a
  parenthesis such as "(not a merge hold)". A section runs to the next heading
  at its level or
  above. A checklist under any other heading is somebody else's (a test plan,
  a gate list) and is left out.
- **Unticked items** (`- [ ]`) inside those. A ticked one is done. That is also
  why nothing here tracks state between batches: an item the user ticks on
  GitHub drops out of the next run by itself.

**Why the REST pull request list and not search.** `gh pr list` is GraphQL and
the search API is global; cloud sessions are refused both, and a cloud session
is exactly who writes a batch report. `repos/{owner}/{repo}/pulls` is answered
everywhere. It is listed by last update, newest first, and a pull request
merged at or after T was updated at or after T too, so paging stops at the
first one updated before T. It also reads the merge time from the pull request
itself, where search would lag its index by a minute.

Every question goes through `mergeable.gh`, so a network blip is retried and an
outage says *unreachable* (exit 3) rather than printing an empty list that
looks like "nothing to check".

    python3 .github/scripts/human-checks.py --since 2026-10-05T18:00:00Z
    make checks SINCE=2026-10-05
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "mergeable", Path(__file__).resolve().with_name("mergeable.py")
)
mergeable = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(mergeable)

# A Markdown ATX heading: its level, and its text without a closing run of `#`.
HEADING = re.compile(r"^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$")

# The heading texts that open a human-check section. Matched at the start of
# the text, case-insensitively, so "Human checklist (not a merge hold)" and
# "For a human, later" are both in. Gathered from the merged pull requests up
# to #818 (#821's pull request lists them); a new spelling goes here. *For the
# maintainer* joined after the 2026-10-06 batch, whose first run of this missed
# the three sections that held its deploy steps (#848, #856, #859).
HUMAN_HEADING = re.compile(
    r"^(for a human|left for a human|human check|for the maintainer)", re.IGNORECASE
)

# An item that says there is nothing to check is not a check. Coders write one
# as a checkbox (#835, #860), and counted, it is a false entry in the list.
NOTHING = re.compile(r"^nothing\b", re.IGNORECASE)

# An unticked task-list item, and its text. A ticked `[x]` never matches.
UNTICKED = re.compile(r"^(\s*)[-*+]\s+\[ \]\s+(.*\S)\s*$")

# Any list item, ticked or not: what ends the continuation of the one before.
LIST_ITEM = re.compile(r"^\s*([-*+]|\d+[.)])\s")

# Where a fenced code block opens or closes. A checklist inside one is an
# example, not a check.
FENCE = re.compile(r"^\s*(```|~~~)")

# How many pull requests one page asks for: the API's maximum.
PAGE = 100


def human_checks(body: str | None) -> list[str]:
    """The unticked items in `body`'s human-check sections, one string each.

    An item that wraps onto indented lines below it is joined into one line, so
    each check reads whole in the list. A nested item stays its own item.
    """
    items: list[str] = []
    inside: int | None = None  # the level of the open human section, if any
    fenced = False
    current: tuple[int, list[str]] | None = None  # indent and text of the open item

    def close() -> None:
        nonlocal current
        if current is not None:
            text = " ".join(current[1])
            if not NOTHING.match(text):
                items.append(text)
            current = None

    for line in (body or "").splitlines():
        if FENCE.match(line):
            fenced = not fenced
            close()
            continue
        if fenced:
            continue
        heading = HEADING.match(line)
        if heading:
            close()
            level, text = len(heading.group(1)), heading.group(2)
            if inside is not None and level <= inside:
                inside = None
            if inside is None and HUMAN_HEADING.match(text):
                inside = level
            continue
        if inside is None:
            continue
        unticked = UNTICKED.match(line)
        if unticked:
            close()
            current = (len(unticked.group(1)), [unticked.group(2)])
        elif current is not None and line.strip() and not LIST_ITEM.match(line) \
                and len(line) - len(line.lstrip()) > current[0]:
            current[1].append(line.strip())
        else:
            close()
    close()
    return items


def moment(text: str) -> datetime:
    """An ISO 8601 date or time, as an aware UTC datetime.

    A bare date means its midnight in UTC, and a time with no offset is read as
    UTC — what GitHub's own timestamps are, so a time copied from one compares
    as it reads.
    """
    parsed = datetime.fromisoformat(text.strip().replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def merged_since(since: datetime, gh=None) -> list[dict]:
    """Every pull request merged at or after `since`, oldest number first.

    `gh` is [`mergeable.gh`] unless a test hands in its own.
    """
    gh = gh or mergeable.gh
    merged: list[dict] = []
    page = 1
    while True:
        pulls = gh(
            "api",
            f"repos/{{owner}}/{{repo}}/pulls?state=closed&sort=updated"
            f"&direction=desc&per_page={PAGE}&page={page}",
        )
        for pull in pulls:
            if pull.get("merged_at") and moment(pull["merged_at"]) >= since:
                merged.append(pull)
        if len(pulls) < PAGE or moment(pulls[-1]["updated_at"]) < since:
            break
        page += 1
    return sorted(merged, key=lambda pull: pull["number"])


def report(pulls: list[dict], since: str) -> str:
    """The Markdown list: a heading per pull request with checks, then a total."""
    lines: list[str] = []
    total = 0
    with_checks = 0
    for pull in pulls:
        checks = human_checks(pull.get("body"))
        if not checks:
            continue
        with_checks += 1
        total += len(checks)
        lines.append(f"### [#{pull['number']}]({pull['html_url']}): {pull['title']}")
        lines.append("")
        lines.extend(f"- [ ] {check}" for check in checks)
        lines.append("")
    if not total:
        return (
            f"No unticked human checks in the {len(pulls)} pull request(s)"
            f" merged since {since}."
        )
    lines.append(
        f"{total} unticked human check(s) in {with_checks} of the"
        f" {len(pulls)} pull request(s) merged since {since}."
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--since",
        required=True,
        help="ISO 8601 date or time (UTC unless it says otherwise), e.g. 2026-10-05T18:00:00Z",
    )
    args = parser.parse_args(argv)
    try:
        since = moment(args.since)
    except ValueError:
        parser.error(f"--since {args.since!r} is not an ISO 8601 date or time")
    try:
        pulls = merged_since(since)
    except mergeable.Unreachable as outage:
        print(f"human-checks: {outage}", file=sys.stderr)
        return mergeable.UNREACHABLE_STATUS
    print(report(pulls, args.since))
    return 0


if __name__ == "__main__":
    sys.exit(main())
