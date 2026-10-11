#!/usr/bin/env python3
"""When a file's survivors are one finding, and when they are a worklist.

Split from `test_mutants_summary.py` (#1136); run as a subprocess the same way.
"""

from __future__ import annotations

import unittest

from mutants_runs import caught, render, survivor


class Collapsing(unittest.TestCase):
    """125 survivors in one module is one finding, and is reported as one."""

    LAYOUT = "crates/compositor/src/text/layout.rs"

    def test_a_module_nothing_was_caught_in_is_a_sentence_not_a_worklist(self) -> None:
        # The shape #290 was filed over: 125 mutants of one file, none caught.
        # Naming them individually presents a foregone conclusion as work.
        out = render(*(survivor(self.LAYOUT, 20 + i, f"draw_{i}", "()") for i in range(125)))

        self.assertIn("125 of 125 mutations survived, nothing caught", out)
        self.assertIn("nothing in the mutated package asserts on this code", out)
        self.assertNotIn("draw_7", out)

    def test_the_nothing_caught_banner_asks_rather_than_answers(self) -> None:
        """#449: the banner used to name a cause, and named the wrong one twice.

        It said "usually structural" — assertions living in another crate. The
        two documented cases were a branch's own untested `serde` defaults
        (#442) and a public method with no callers at all (#444). So the
        banner has to offer all three causes without ranking one, and to send
        the reader at the command that decides between them rather than at
        `.cargo/mutants.toml`.
        """
        out = render(*(survivor(self.LAYOUT, 20 + i, f"draw_{i}", "()") for i in range(125)))

        self.assertNotIn("usually structural", out)
        # All three causes, and the third named as such rather than as the one.
        self.assertIn("no test", out)
        self.assertIn("no callers", out)
        self.assertIn("test_workspace = false", out)
        # Deletion is a legitimate outcome, and the cheap check comes first.
        self.assertIn("deleting it is the fix", out)
        self.assertIn("cargo mutants --list", out)

    def test_a_long_survivor_list_collapses_even_where_something_was_caught(self) -> None:
        # The tests do reach this file — but past a certain
        # length the rows stop being read, so the count is what is reported.
        out = render(
            caught(self.LAYOUT, 10),
            *(survivor(self.LAYOUT, 20 + i, f"draw_{i}", "()") for i in range(12)),
        )

        self.assertIn("12 of 13 mutations survived, too many to read", out)
        self.assertNotIn("nothing caught", out)
        self.assertNotIn("draw_7", out)

    def test_a_short_scattered_list_is_still_listed_row_by_row(self) -> None:
        # The report has to keep doing its job in the ordinary case: a few
        # survivors across a few files are the worklist, and each row names the
        # mutation the reader has to judge.
        out = render(
            caught("crates/core/src/keyframe.rs", 40),
            survivor("crates/core/src/keyframe.rs", 61, "evaluate", "0.0"),
            survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="),
        )

        self.assertIn("crates/core/src/keyframe.rs:61", out)
        self.assertIn("crates/core/src/timeline.rs:184", out)
        self.assertIn("`overlaps`", out)
        self.assertNotIn("one finding each", out)

    def test_one_unopposed_survivor_is_not_yet_a_whole_file_finding(self) -> None:
        # A single mutant surviving is a single mutant surviving. Calling a
        # file unasserted on that evidence would hide the one row that says
        # what actually happened.
        out = render(survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="))

        self.assertIn("crates/core/src/timeline.rs:184", out)
        self.assertNotIn("nothing caught", out)

    def test_nothing_is_paginated_away(self) -> None:
        # What `MAX_ROWS = 25` used to do. Twenty-six survivors spread thinly
        # are twenty-six things to look at, and the last of them is not a
        # footnote.
        files = [f"crates/core/src/m{i}.rs" for i in range(26)]
        out = render(*(survivor(f, 7, "evaluate", "0.0") for f in files))

        self.assertNotIn("more |", out)
        for file in files:
            self.assertIn(f"{file}:7", out)


if __name__ == "__main__":
    unittest.main()
