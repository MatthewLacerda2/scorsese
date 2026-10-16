#!/usr/bin/env python3
"""What the report says about the mutations it did not measure (#399).

Split from `test_summary.py` (#1136); run as a subprocess the same way.
"""

from __future__ import annotations

import unittest

from mutation.runs import caught, report


class WhatTheReportDoesNotCover(unittest.TestCase):
    """#399: a diff too large to measure must say so, not vanish.

    The count comes from `cargo mutants --list --in-diff`, taken before
    anything is built — which is exactly why it is still available when the run
    itself is stopped. Everything here is about the difference between it and
    what came back.
    """

    def test_the_shortfall_is_named_in_mutations_and_not_implied(self) -> None:
        # The whole complaint in #399: the honest outcome used to render as a
        # cancelled job, which reads as a broken one. It now reads as a number.
        out = report(
            *(caught("crates/core/src/keyframe.rs", 40 + i) for i in range(96)),
            args=("--in-scope", "233"),
        )

        self.assertIn("137 of 233 mutations in scope were not measured", out)
        self.assertIn("describes 96", out)

    def test_a_report_that_covered_everything_says_nothing_about_a_gap(self) -> None:
        # The ordinary case is every pull request. A banner that appeared there
        # too would be a banner nobody reads by the third one.
        out = report(
            caught("crates/core/src/keyframe.rs", 40),
            caught("crates/core/src/keyframe.rs", 41),
            args=("--in-scope", "2"),
        )

        self.assertNotIn("were not measured", out)

    def test_the_baseline_is_not_counted_as_a_mutation_that_was_measured(self) -> None:
        # cargo-mutants records its unmutated baseline as an outcome whose
        # scenario is the string "Baseline". Counting it — once per shard —
        # would hide a real gap behind a run that looks bigger than it is.
        out = report(
            {"summary": "Success", "scenario": "Baseline"},
            caught("crates/core/src/keyframe.rs", 40),
            args=("--in-scope", "2"),
        )

        self.assertIn("1 of 2 mutations in scope were not measured", out)

    def test_a_stopped_run_says_both_that_it_stopped_and_by_how_much(self) -> None:
        # Two different claims. One says the run did not get to the end; the
        # other says how much of the diff nobody looked at. A reader acting on
        # the report needs the second, and only the second is a number.
        out = report(
            caught("crates/core/src/keyframe.rs", 40),
            args=("--in-scope", "50"),
            start_time="2026-08-26T04:00:00Z",
        )

        self.assertIn("did not finish", out)
        self.assertIn("49 of 50 mutations", out)

    def test_without_the_count_the_report_is_what_it_always_was(self) -> None:
        # The sweep does not pass one, and must go on rendering unchanged.
        out = report(caught("crates/core/src/keyframe.rs", 40))

        self.assertNotIn("were not measured", out)
        self.assertIn("All 1 mutations were caught", out)


if __name__ == "__main__":
    unittest.main()
