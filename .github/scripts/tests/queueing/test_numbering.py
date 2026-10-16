#!/usr/bin/env python3
"""The queue hands back a number `main` already holds, before it pushes (#729).

Two branches cut from the same `main` count up to the same migration number,
the same `SCHEMA_VERSION` or the same `SYNTH_VERSION` (#1216), and git rebases
both cleanly. On 2026-10-04 #714 and
#716 each added `crates/server/migrations/0017_*.sql`; a person caught it by
reading. [`queue.numbering`] is that reading, tested without git.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "merge-queue.py"

_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPT)
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)

# Loading the script put its directory on the path; the package is read
# directly for what the script itself has no use to re-export.
from merge_queue import numbering  # noqa: E402

DIR = "crates/server/migrations"
MAIN = [f"{DIR}/0016_voice_designs.sql", f"{DIR}/0017_library_midi.sql", f"{DIR}/README.md"]
UNTOUCHED = (43, 43, 43, 43)


class Migrations(unittest.TestCase):
    def test_a_number_main_already_holds_is_handed_back_with_the_next_one(self):
        lines = queue.numbering(MAIN, [f"{DIR}/0017_dollars_only.sql"], UNTOUCHED)
        self.assertEqual(len(lines), 1)
        self.assertIn("0017_dollars_only.sql", lines[0])
        self.assertIn("renumber from 18", lines[0])

    def test_the_next_free_number_passes(self):
        self.assertEqual(
            queue.numbering(MAIN, [f"{DIR}/0018_dollars_only.sql"], UNTOUCHED), []
        )

    def test_two_of_the_branchs_own_on_one_number_collide_with_each_other(self):
        added = [f"{DIR}/0018_a.sql", f"{DIR}/0018_b.sql"]
        lines = queue.numbering(MAIN, added, UNTOUCHED)
        self.assertIn("0018_a.sql", lines[0])
        self.assertIn("0018_b.sql", lines[0])

    def test_a_file_that_is_not_a_numbered_migration_is_not_counted(self):
        self.assertEqual(queue.migration_number(f"{DIR}/README.md"), None)
        self.assertEqual(queue.migration_number(f"{DIR}/notes_0017.sql"), None)
        self.assertEqual(queue.migration_number(f"{DIR}/0017_x.sql"), 17)
        self.assertEqual(queue.numbering(MAIN, [f"{DIR}/README.md"], UNTOUCHED), [])


class SchemaVersion(unittest.TestCase):
    def test_a_bump_main_already_made_is_handed_back_with_the_next_number(self):
        # Both sides bumped 43 -> 44: the identical edit, so no conflict.
        lines = queue.numbering(MAIN, [], (43, 44, 44, 44))
        self.assertEqual(len(lines), 1)
        self.assertIn("Renumber to 45", lines[0])

    def test_a_bump_main_has_not_made_passes(self):
        self.assertEqual(queue.numbering(MAIN, [], (43, 44, 44, 43)), [])

    def test_a_branch_that_never_bumped_is_not_blamed_for_mains_bump(self):
        self.assertEqual(queue.numbering(MAIN, [], (43, 43, 44, 44)), [])

    def test_an_unreadable_version_is_no_evidence(self):
        self.assertEqual(queue.numbering(MAIN, [], (None, 44, None, None)), [])

    def test_the_constant_is_read_from_its_declaration(self):
        source = "//! doc\npub const SCHEMA_VERSION: u32 = 43;\n"
        self.assertEqual(queue.schema_version(source), 43)
        self.assertEqual(queue.schema_version("fn main() {}"), None)
        self.assertEqual(queue.schema_version(None), None)


class SynthVersion(unittest.TestCase):
    """The silent one: a shared bump ships the second branch's sound change
    under the first's number, so every bake of that number goes stale."""

    def test_a_bump_main_already_made_is_handed_back_with_the_next_number(self):
        lines = queue.numbering(MAIN, [], UNTOUCHED, synth=(8, 9, 9, 9))
        self.assertEqual(len(lines), 1)
        self.assertIn("`SYNTH_VERSION` 9", lines[0])
        self.assertIn("Renumber to 10", lines[0])
        self.assertIn("SYNTH_VERSION.md", lines[0])

    def test_a_bump_main_has_not_made_passes(self):
        self.assertEqual(queue.numbering(MAIN, [], UNTOUCHED, synth=(8, 9, 9, 8)), [])

    def test_a_branch_that_never_bumped_is_not_blamed_for_mains_bump(self):
        self.assertEqual(queue.numbering(MAIN, [], UNTOUCHED, synth=(8, 8, 9, 9)), [])

    def test_a_caller_with_no_reading_passes(self):
        self.assertEqual(queue.numbering(MAIN, [], UNTOUCHED), [])

    def test_the_constant_is_read_from_its_declaration(self):
        source = "/// The table.\npub const SYNTH_VERSION: u32 = 9;\n"
        self.assertEqual(numbering.synth_version(source), 9)
        self.assertEqual(numbering.synth_version("pub const SCHEMA_VERSION: u32 = 9;"), None)
        self.assertEqual(numbering.synth_version(None), None)

    def test_the_paths_read_are_the_ones_that_declare_the_constants(self):
        # A moved file reads as no evidence, so the check would pass every
        # branch without a word: `project.rs` became `project/mod.rs` in #1295
        # and the `SCHEMA_VERSION` check went blind until this test (#1216).
        root = Path(__file__).resolve().parents[4]
        for path, read in (
            (numbering.SYNTH_FILE, numbering.synth_version),
            (numbering.SCHEMA_FILE, numbering.schema_version),
        ):
            self.assertIsNotNone(read((root / path).read_text()), path)


class Both(unittest.TestCase):
    def test_both_collisions_are_named_together(self):
        lines = queue.numbering(
            MAIN, [f"{DIR}/0017_x.sql"], (43, 44, 44, 44), synth=(8, 9, 9, 9)
        )
        self.assertEqual(len(lines), 3)


if __name__ == "__main__":
    unittest.main()
