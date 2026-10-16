#!/usr/bin/env python3
"""Every environment variable the code reads is written down somewhere.

A variable nobody documented is a setting nobody can find: `SCORSESE_FFPROBE`
was read by `scorsese-render` and named on no page, and `SCORSESE_FFMPEG` only
in `CLAUDE.md`, which a user never reads (#1226). Three holds:

- every `SCORSESE_*`, `*_API_KEY` and `DATABASE_URL` the code names is in
  `.env.example`, `README.md` or a page under `docs/`;
- `docs/web.md`'s configuration table has a row for every variable
  `crates/server/src/config.rs` declares, and no row for one the code never
  reads;
- every `NAME=` line in `.env.example` is one the code reads.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SOURCES = ("crates/*/src/**/*.rs", "app/src/**/*.rs")
# A string literal naming a variable the issue's convention covers.
READ = re.compile(r'"(SCORSESE_[A-Z0-9_]+|[A-Z0-9_]+_API_KEY|DATABASE_URL)"')
# `pub const NAME: &str = "VARIABLE";` in config.rs, its values only.
DECLARED = re.compile(r'^pub const [A-Z_]+: &str = "([A-Z][A-Z0-9_]+)";', re.M)
TABLE_HEADER = "| Variable | What it is |"


def read_by(sources: dict[str, str]) -> set[str]:
    return {m.group(1) for text in sources.values() for m in READ.finditer(text)}


def table_rows(web: str) -> set[str]:
    """The variables named in the first cell of docs/web.md's configuration table."""
    lines = web.splitlines()
    try:
        start = lines.index(TABLE_HEADER)
    except ValueError:
        return set()
    names: set[str] = set()
    for line in lines[start + 2 :]:
        if not line.startswith("|"):
            break
        first = line.split("|")[1]
        names |= set(re.findall(r"`([A-Z][A-Z0-9_]+)`", first))
    return names


def example_names(example: str) -> set[str]:
    return set(re.findall(r"^([A-Z][A-Z0-9_]+)=", example, re.M))


def problems(read: set[str], declared: set[str], docs: dict[str, str], example: str) -> list[str]:
    found: list[str] = []
    for name in sorted(read):
        if not any(re.search(rf"\b{name}\b", text) for text in docs.values()):
            found.append(f"{name} is read by the code and documented on no page")
    rows = table_rows(docs.get("docs/web.md", ""))
    found += [f"docs/web.md's configuration table has no row for {n}" for n in sorted(declared - rows)]
    found += [f"docs/web.md's configuration table names {n}, which nothing reads" for n in sorted(rows - read - declared)]
    found += [f".env.example sets {n}, which nothing reads" for n in sorted(example_names(example) - read - declared)]
    return found


class EnvDocumented(unittest.TestCase):
    def test_every_variable_read_is_documented(self) -> None:
        sources = {
            str(p): p.read_text(encoding="utf-8") for pattern in SOURCES for p in ROOT.glob(pattern)
        }
        config = (ROOT / "crates/server/src/config.rs").read_text(encoding="utf-8")
        docs = {
            str(p.relative_to(ROOT)): p.read_text(encoding="utf-8")
            for p in [ROOT / ".env.example", ROOT / "README.md", *sorted((ROOT / "docs").glob("*.md"))]
        }
        self.assertEqual(
            problems(read_by(sources), set(DECLARED.findall(config)), docs, docs[".env.example"]), []
        )

    def test_an_undocumented_or_stale_variable_is_named(self) -> None:
        web = f"intro\n\n{TABLE_HEADER}\n|---|---|\n| `SCORSESE_BIND` | x |\n| `OLD_ONE`, `KEPT` | y |\n\nafter `LATER`\n"
        docs = {"docs/web.md": web, "README.md": "set `SCORSESE_FFMPEG`"}
        self.assertEqual(
            problems(
                {"SCORSESE_BIND", "SCORSESE_FFMPEG", "SCORSESE_FFPROBE", "KEPT"},
                {"SCORSESE_BIND", "KEPT", "GOOGLE_ID"},
                docs,
                "# c\nKEPT=\nGONE=1\n",
            ),
            [
                "SCORSESE_FFPROBE is read by the code and documented on no page",
                "docs/web.md's configuration table has no row for GOOGLE_ID",
                "docs/web.md's configuration table names OLD_ONE, which nothing reads",
                ".env.example sets GONE, which nothing reads",
            ],
        )

    def test_the_code_s_literals_and_declarations_are_read(self) -> None:
        source = 'let a = "SCORSESE_CACHE"; let b = "GEMINI_API_KEY"; let c = "UPDATE_X";'
        self.assertEqual(read_by({"a.rs": source}), {"SCORSESE_CACHE", "GEMINI_API_KEY"})
        config = 'pub const BIND: &str = "SCORSESE_BIND";\npub const DEFAULT_BIND: &str = "127.0.0.1:8080";\n'
        self.assertEqual(DECLARED.findall(config), ["SCORSESE_BIND"])


if __name__ == "__main__":
    unittest.main()
