"""`mutants/exclusions.py`: every `exclude_re` entry has a reason, escapes
its operators, and names its line (#1229) — the real file, and the cases each
rule must flag and pass."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "mutants" / "exclusions.py"
_spec = importlib.util.spec_from_file_location("exclusions", SCRIPT)
exclusions = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(exclusions)


def config(*lines: str) -> str:
    return "\n".join(["examine_globs = []", "exclude_re = [", *lines, "]", ""])


GOOD = r"'blur\.rs:106:24: replace \|\| with && in radius',"


class TheRealFile(unittest.TestCase):
    def test_every_entry_in_mutants_toml_keeps_the_rules(self):
        text = exclusions.DEFAULT.read_text(encoding="utf-8")
        self.assertGreater(len(exclusions.entries(text)), 40, "the parser stopped reading it")
        self.assertEqual(exclusions.problems(text), [])


class TheRules(unittest.TestCase):
    def test_a_commented_qualified_escaped_entry_passes(self):
        self.assertEqual(exclusions.problems(config("    # Equivalent: why.", "    " + GOOD)), [])

    def test_a_run_of_entries_shares_the_comment_above_it(self):
        text = config("    # Both equivalent.", "    " + GOOD, "    " + GOOD)
        self.assertEqual(exclusions.problems(text), [])

    def test_an_entry_with_no_comment_is_flagged(self):
        text = config("    # Reason.", "    " + GOOD, "", "    " + GOOD)
        found = exclusions.problems(text)
        self.assertEqual(len(found), 1, found)
        self.assertIn("no comment", found[0])

    def test_an_unescaped_operator_is_flagged_and_a_class_is_not(self):
        text = config("    # Reason.", r"    'blur\.rs:1:2: replace || with && in radius',")
        self.assertIn("unescaped |", " ".join(exclusions.problems(text)))
        self.assertEqual(exclusions.unescaped(r"Mix<[^>|]a> \+ \* \?"), [])
        self.assertEqual(exclusions.unescaped(r"a + b * c?"), ["+", "*", "?"])

    def test_an_entry_without_its_line_is_flagged(self):
        for pattern in (r"replace \+ with - in advance", r"mix\.rs:\d+:\d+: replace sends"):
            text = config("    # Reason.", f"    '{pattern}',")
            self.assertIn("no line", " ".join(exclusions.problems(text)), pattern)


if __name__ == "__main__":
    unittest.main()
