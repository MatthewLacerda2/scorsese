#!/usr/bin/env python3
"""Which changes `ci.yml` runs for, read from `ci.yml` itself (#1040, #1135).

CI skips a Markdown-only change, except for the pages a build or a test reads,
which `ci.yml`'s `paths:` lists re-include by name. Two readers need that list
and must never disagree about it: `tests/test_ci_read_docs.py`, which holds it
to the code, and `mergeable.py`, which says yes to a pull request with no run
only when the list says CI was never going to make one. So the list is parsed
here, once, and both import it.

A line-wise read rather than a YAML parser: this is stdlib-only, and the `on:`
block is a fixed, shallow shape.

[`runs_for`] follows GitHub's own rule for a `paths:` filter: the patterns are
read in order, the last one a file matches decides whether it counts, a `!`
excludes, and CI runs when any changed file counts. `*` stops at a `/` and
`**` does not, so `!**.md` is every Markdown file at any depth.
"""

from __future__ import annotations

import re


def paths(workflow: str) -> dict[str, list[str]]:
    """Each trigger's `paths:` patterns, in order, by trigger name."""
    found: dict[str, list[str]] = {}
    trigger = None
    in_paths = False
    for line in workflow.splitlines():
        if line and not line[0].isspace() and line.rstrip() != "on:":
            if found:
                break
            continue
        head = re.match(r"^  (\w+):", line)
        if head:
            trigger, in_paths = head.group(1), False
            continue
        if line.strip() == "paths:":
            in_paths = True
            found.setdefault(trigger, [])
            continue
        item = re.match(r'^\s+- "([^"]+)"$', line)
        if in_paths and item:
            found[trigger].append(item.group(1))
        elif in_paths and line.strip():
            in_paths = False
    return found


def exceptions(workflow: str) -> dict[str, set[str]]:
    """Each trigger's re-included Markdown pages, by trigger name.

    A pattern is a page when it names one `.md` file — not `**`, not a `!`
    exclusion.
    """
    return {
        trigger: {
            pattern
            for pattern in patterns
            if pattern.endswith(".md")
            and "*" not in pattern
            and not pattern.startswith("!")
        }
        for trigger, patterns in paths(workflow).items()
    }


def glob(pattern: str) -> re.Pattern[str]:
    """A `paths:` pattern as a regular expression over a whole path."""
    out = []
    for token in re.split(r"(\*\*|\*|\?)", pattern):
        if token == "**":
            out.append(".*")
        elif token == "*":
            out.append("[^/]*")
        elif token == "?":
            out.append("[^/]")
        else:
            out.append(re.escape(token))
    return re.compile("".join(out) + r"\Z")


def runs_for(patterns: list[str], files: list[str]) -> bool:
    """Whether a change to `files` passes a trigger's `paths:` filter.

    No patterns at all means no filter, and every change runs.
    """
    if not patterns:
        return True
    compiled = [
        (pattern.startswith("!"), glob(pattern.lstrip("!"))) for pattern in patterns
    ]
    for name in files:
        counts = False
        for excluded, regex in compiled:
            if regex.match(name):
                counts = not excluded
        if counts:
            return True
    return False


def skips(workflow: str, files: list[str]) -> bool:
    """Whether `ci.yml` makes no run at all for a pull request changing `files`.

    Asked of the `pull_request` trigger, since that is the one a pull request's
    run comes from. An empty change is not called skipped: there is nothing to
    say it was Markdown.
    """
    triggers = paths(workflow)
    if not files or "pull_request" not in triggers:
        return False
    return not runs_for(triggers["pull_request"], files)
