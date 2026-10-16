#!/usr/bin/env python3
"""Every page the code reads is one CI's jobs run for (#1040, #1351).

`ci.yml`'s jobs skip a Markdown-only change, with an exception list of the
pages a build or a test reads (`ci-paths.py`'s `PATTERNS`, which the `changes`
job asks): a page embedded by `include_str!`, or opened by a test, is code in
all but name, and a Markdown-only edit that breaks it would merge with nothing
compiled and turn the next code pull request red for a reason its diff does
not contain. The list is written by hand, so this keeps it from drifting:

- every `.md` file a Rust source names by a path that resolves — from the
  source's own folder, as `include_str!` resolves, or from its crate's folder,
  as `CARGO_MANIFEST_DIR` and a test's working directory do — is listed;
- every listed page exists, so a rename does not leave a dead exception behind.

A page named only by a file name that is not a path to it (the migrations
folder's `README.md`, which a test skips by name) is not read, and is not held.
"""

from __future__ import annotations

import importlib.util
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# The list is `ci.yml`'s `changes` job's too (#1351), so both read it from
# the one place it is written.
_spec = importlib.util.spec_from_file_location(
    "ci_paths", ROOT / ".github" / "scripts" / "ci-paths.py"
)
ci_paths = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ci_paths)
exceptions = ci_paths.exceptions
SOURCES = ("crates", "app", "tools")
LITERAL = re.compile(r'"([^"\s{}]*\.md)"')


def crate_dir(source: Path) -> Path:
    """The nearest folder above `source` holding a `Cargo.toml`."""
    for parent in source.parents:
        if (parent / "Cargo.toml").is_file():
            return parent
    return ROOT


def read_pages() -> dict[str, str]:
    """Every Markdown file in the repository a Rust source names, and who names it."""
    pages: dict[str, str] = {}
    for top in SOURCES:
        for source in sorted((ROOT / top).rglob("*.rs")):
            if "target" in source.relative_to(ROOT).parts:
                continue
            text = source.read_text(encoding="utf-8")
            for literal in LITERAL.findall(text):
                relative = literal.lstrip("/")
                for base in (source.parent, crate_dir(source)):
                    page = (base / relative).resolve()
                    if page.is_file() and ROOT in page.parents:
                        name = page.relative_to(ROOT).as_posix()
                        pages.setdefault(name, source.relative_to(ROOT).as_posix())
    return pages


class ReadDocs(unittest.TestCase):
    def setUp(self) -> None:
        self.listed = exceptions()

    def test_every_page_the_code_reads_is_listed(self) -> None:
        pages = read_pages()
        self.assertIn("docs/project-format.md", pages, "the scan found nothing it should")
        missing = [
            f"{source} reads {page}, but ci-paths.py's PATTERNS does not "
            "re-include it, so a Markdown-only edit to it compiles nothing"
            for page, source in sorted(pages.items())
            if page not in self.listed
        ]
        self.assertEqual(missing, [], "\n".join(missing))

    def test_every_listed_page_exists(self) -> None:
        for page in sorted(self.listed):
            self.assertTrue(
                (ROOT / page).is_file(),
                f"ci-paths.py's PATTERNS re-includes {page}, which is not there",
            )


class Parsing(unittest.TestCase):
    def test_only_named_pages_count(self) -> None:
        patterns = ("**", "!**.md", "docs/a.md", "docs/*.md", "!docs/b.md")
        self.assertEqual(exceptions(patterns), {"docs/a.md"})


if __name__ == "__main__":
    unittest.main()
