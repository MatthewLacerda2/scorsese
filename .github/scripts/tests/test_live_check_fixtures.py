#!/usr/bin/env python3
"""`docs/live-check.md`'s "Which fixtures are real" table covers every fixture.

That table is how a reader tells a response shape scorsese has seen a vendor
send from one somebody wrote by hand. A fixture with no row is a shape whose
provenance nobody can check — `pixabay/images.json` and `pixabay/videos.json`
had none (#1226) — and a row naming no file points at a fixture that is gone.
Rows name files relative to `crates/providers/`, with `{a,b}` and `*` as a
shell would read them.
"""

from __future__ import annotations

import fnmatch
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PROVIDERS = ROOT / "crates/providers"
DOC = ROOT / "docs/live-check.md"
HEADING = "## Which fixtures are real"


def expand(pattern: str) -> list[str]:
    """`a/{b,c}.json` → `a/b.json`, `a/c.json`; one brace group is all the table uses."""
    braces = re.search(r"\{([^{}]*)\}", pattern)
    if braces is None:
        return [pattern]
    return [
        pattern[: braces.start()] + part + pattern[braces.end() :]
        for part in braces.group(1).split(",")
    ]


def named(doc: str) -> list[str]:
    """Every `fixtures/...` the section's table names, braces expanded."""
    section = doc.split(HEADING, 1)[1].split("\n## ", 1)[0] if HEADING in doc else ""
    return [
        each
        for line in section.splitlines()
        if line.startswith("|")
        for found in re.findall(r"`(fixtures/[^`]+)`", line)
        for each in expand(found)
    ]


def problems(patterns: list[str], files: list[str]) -> list[str]:
    found = [f"{p} is in the table and matches no fixture" for p in patterns if not fnmatch.filter(files, p)]
    found += [
        f"{f} has no row in the table"
        for f in files
        if not any(fnmatch.fnmatchcase(f, p) for p in patterns)
    ]
    return found


class LiveCheckFixtures(unittest.TestCase):
    def test_every_fixture_has_a_row_and_every_row_a_fixture(self) -> None:
        files = sorted(
            str(p.relative_to(PROVIDERS)) for p in (PROVIDERS / "fixtures").rglob("*") if p.is_file()
        )
        self.assertEqual(problems(named(DOC.read_text(encoding="utf-8")), files), [])

    def test_braces_and_globs_are_read_as_a_shell_would(self) -> None:
        doc = f"{HEADING}\n\n| a | `x` | `fixtures/veo/{{running,done}}.json` |\n| b | `fixtures/anthropic/*.sse` |\n\n## Next\n| `fixtures/not/this.json` |\n"
        self.assertEqual(
            named(doc),
            ["fixtures/veo/running.json", "fixtures/veo/done.json", "fixtures/anthropic/*.sse"],
        )

    def test_a_fixture_with_no_row_and_a_row_with_no_fixture_are_named(self) -> None:
        files = ["fixtures/anthropic/answer.sse", "fixtures/pixabay/images.json"]
        self.assertEqual(
            problems(["fixtures/anthropic/*.sse", "fixtures/veo/gone.json"], files),
            [
                "fixtures/veo/gone.json is in the table and matches no fixture",
                "fixtures/pixabay/images.json has no row in the table",
            ],
        )


if __name__ == "__main__":
    unittest.main()
