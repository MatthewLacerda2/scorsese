#!/usr/bin/env python3
"""Every docs page the server binary embeds is one its image lets in.

`deploy/server.Dockerfile.dockerignore` is an allow-list, and `docs/` is
mostly left out of it: only the pages a crate compiles in with `include_str!`
are let through. A page added to the code and not to the list builds
everywhere except inside the image, and nothing else runs that build — on
2026-10-10 the deploy after the batch failed on `docs/styles.md` (#1002), which
`scorsese-core`'s `guide` had compiled in since that morning.

So: every `docs/*.md` an `include_str!` names in a crate's `src/`, outside a
test, is in the list, and every page the list names exists. "Outside a test"
is read plainly: a file under a `tests` folder, a `tests.rs`, or an
`include_str!` below the file's first inline `#[cfg(test)] mod … {` block is a
test's. (`#[cfg(test)] mod pointers;` above the code only names another file.)
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
IGNORE = ROOT / "deploy" / "server.Dockerfile.dockerignore"
EMBED = re.compile(r'include_str!\(\s*"([^"]+\.md)"\s*\)')
TEST_BLOCK = re.compile(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{")


def allowed(text: str) -> set[str]:
    """The `docs/` pages the allow-list lets in."""
    return {
        line[1:].strip()
        for line in text.splitlines()
        if line.startswith("!docs/") and line.strip().endswith(".md")
    }


def embedded() -> dict[str, str]:
    """Every docs page a crate's non-test source compiles in, and who does."""
    pages: dict[str, str] = {}
    for source in sorted((ROOT / "crates").glob("*/src/**/*.rs")):
        parts = source.relative_to(ROOT).parts
        if "tests" in parts or source.name == "tests.rs":
            continue
        text = source.read_text(encoding="utf-8")
        block = TEST_BLOCK.search(text)
        for found in EMBED.finditer(text):
            if block and found.start() > block.start():
                continue
            page = (source.parent / found.group(1)).resolve()
            if page.is_file() and ROOT in page.parents:
                name = page.relative_to(ROOT).as_posix()
                if name.startswith("docs/"):
                    pages.setdefault(name, source.relative_to(ROOT).as_posix())
    return pages


class ServerImageDocs(unittest.TestCase):
    def setUp(self) -> None:
        self.allowed = allowed(IGNORE.read_text(encoding="utf-8"))

    def test_every_embedded_page_is_let_in(self) -> None:
        pages = embedded()
        self.assertIn("docs/project-format.md", pages, "the scan found nothing it should")
        missing = [
            f"{source} compiles in {page}, but deploy/server.Dockerfile.dockerignore "
            f"does not let it in: add `!{page}`, or the server image will not build"
            for page, source in sorted(pages.items())
            if page not in self.allowed
        ]
        self.assertEqual(missing, [], "\n".join(missing))

    def test_every_listed_page_exists(self) -> None:
        for page in sorted(self.allowed):
            self.assertTrue((ROOT / page).is_file(), f"the allow-list names {page}, which is not there")


class Parsing(unittest.TestCase):
    def test_only_docs_pages_count(self) -> None:
        text = "*\n!Cargo.toml\n!docs/a.md\n# !docs/b.md\n!tools/x/\n"
        self.assertEqual(allowed(text), {"docs/a.md"})


if __name__ == "__main__":
    unittest.main()
