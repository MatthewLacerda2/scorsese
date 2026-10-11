#!/usr/bin/env python3
"""Every page the code reads is one CI runs for (#1040).

`ci.yml` skips Markdown-only changes, with an exception list of the pages a
build or a test reads: a page embedded by `include_str!`, or opened by a test,
is code in all but name, and a Markdown-only edit that breaks it would merge
with no run and turn the next code pull request red for a reason its diff does
not contain. The list is written by hand, so this keeps it from drifting:

- every `.md` file a Rust source names by a path that resolves — from the
  source's own folder, as `include_str!` resolves, or from its crate's folder,
  as `CARGO_MANIFEST_DIR` and a test's working directory do — is listed;
- every listed page exists, so a rename does not leave a dead exception behind;
- `push` and `pull_request` list the same pages.

A page named only by a file name that is not a path to it (the migrations
folder's `README.md`, which a test skips by name) is not read, and is not held.
"""

from __future__ import annotations

import importlib.util
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# The parser is `mergeable.py`'s too (#1135), so it lives beside it and both
# read the same list the same way.
_spec = importlib.util.spec_from_file_location(
    "ci_paths", ROOT / ".github" / "scripts" / "ci-paths.py"
)
ci_paths = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ci_paths)
exceptions = ci_paths.exceptions
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
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
        self.listed = exceptions(WORKFLOW.read_text(encoding="utf-8"))

    def test_both_triggers_list_the_same_pages(self) -> None:
        self.assertEqual(set(self.listed), {"push", "pull_request"})
        self.assertEqual(self.listed["push"], self.listed["pull_request"])

    def test_every_page_the_code_reads_is_listed(self) -> None:
        pages = read_pages()
        self.assertIn("docs/project-format.md", pages, "the scan found nothing it should")
        missing = [
            f"{source} reads {page}, but ci.yml's `{trigger}: paths:` does not "
            "re-include it, so a Markdown-only edit to it runs no CI"
            for page, source in sorted(pages.items())
            for trigger, listed in sorted(self.listed.items())
            if page not in listed
        ]
        self.assertEqual(missing, [], "\n".join(missing))

    def test_every_listed_page_exists(self) -> None:
        for trigger, listed in self.listed.items():
            for page in sorted(listed):
                self.assertTrue(
                    (ROOT / page).is_file(),
                    f"ci.yml's `{trigger}: paths:` re-includes {page}, which is not there",
                )


class Parsing(unittest.TestCase):
    def test_only_named_pages_count(self) -> None:
        workflow = 'on:\n  push:\n    branches: [main]\n    paths:\n      - "**"\n' \
            '      - "!**.md"\n      - "docs/a.md"\n  pull_request:\n    paths:\n' \
            '      - "docs/a.md"\n      - "docs/b.md"\n\nenv:\n  X: "y.md"\n'
        self.assertEqual(
            exceptions(workflow),
            {"push": {"docs/a.md"}, "pull_request": {"docs/a.md", "docs/b.md"}},
        )


if __name__ == "__main__":
    unittest.main()
