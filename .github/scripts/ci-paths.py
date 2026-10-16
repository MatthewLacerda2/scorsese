#!/usr/bin/env python3
"""Which changes `ci.yml`'s jobs run for (#1040, #1135, #1351).

Every ready pull request and every push to `main` gets a CI run (#1351), so
that its final `CI ok` job always reports and the `main` ruleset can require
it. What a Markdown-only change skips is the **jobs**: the first one,
`changes`, pipes the changed paths into this script, and every heavy job reads
the `code=` line it prints. The pages a build or a test reads are re-included
by name in [`PATTERNS`], because a page embedded by `include_str!` or opened by
a test is code in all but name.

So the list lives here, once, and two readers use it: `ci.yml`'s `changes`
job, and `tests/test_ci_read_docs.py`, which holds it to the code. Before
#1351 it was the workflow's `on: paths:` filter, which made no run at all for
a Markdown-only change, and a required check that never reports leaves a pull
request pending forever.

[`runs_for`] follows GitHub's rule for a `paths:` filter, which is what the list
was written for: the patterns are read in order, the last one a file matches
decides whether it counts, a `!` excludes, and the jobs run when any changed
file counts. `*` stops at a `/` and `**` does not, so `!**.md` is every
Markdown file at any depth.

    git diff --name-only --no-renames BASE HEAD | python3 .github/scripts/ci-paths.py
"""

from __future__ import annotations

import re
import sys

# Ordered: a later pattern overrides an earlier one for a path both match.
# `docs/project-format.md`'s JSON examples are parsed and its
# animatable-property table is compared to the code; `docs/mcp.md`'s tool table
# is generated from the registry; `docs/web.md`'s route tables are held to the
# server's router (#994); and the guides an agent is served (`scorsese guide`)
# are `include_str!`'d into the binary, with tests reading their examples and
# pointers. A page nothing reads stays off, so editing it is still the fast
# path: a run of `changes` and `CI ok`, and nothing compiled.
PATTERNS = (
    "**",
    "!**.md",
    "docs/mcp.md",
    "docs/output-formats.md",
    "docs/pages.md",
    "docs/prices.md",
    "docs/project-format.md",
    "docs/prompts.md",
    "docs/recipes.md",
    "docs/references.md",
    "docs/stock.md",
    "docs/styles.md",
    "docs/web.md",
)


def exceptions(patterns: tuple[str, ...] = PATTERNS) -> set[str]:
    """The re-included Markdown pages: a pattern naming one `.md` file.

    Not `**`, not a `!` exclusion.
    """
    return {
        pattern
        for pattern in patterns
        if pattern.endswith(".md") and "*" not in pattern and not pattern.startswith("!")
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


def runs_for(patterns: tuple[str, ...] | list[str], files: list[str]) -> bool:
    """Whether a change to `files` passes the filter `patterns` spell.

    No patterns at all means no filter, and every change runs. No files at all
    runs too: an empty diff is nothing to say the change was Markdown, and a
    diff that could not be read is not one that may be skipped.
    """
    if not patterns or not files:
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


def main(lines: list[str]) -> str:
    """The `changes` job's output line for the changed paths in `lines`."""
    files = [line.strip() for line in lines if line.strip()]
    return f"code={'true' if runs_for(PATTERNS, files) else 'false'}"


if __name__ == "__main__":
    print(main(sys.stdin.readlines()))
