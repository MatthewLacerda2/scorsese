#!/usr/bin/env python3
"""The sweep's issue body: a report rewritten in place over a history that is not.

Split from `test_mutants_summary.py` (#1136); run as a subprocess the same way.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from mutants_runs import caught, outcomes, render, run, survivor, sweep


class TheSweepsIssueBody(unittest.TestCase):
    """One issue, rewritten in place — except the part that must not be."""

    def test_the_report_is_the_report_the_pull_request_comment_gets(self) -> None:
        # Why there is one renderer and not two. If the sweep's copy could say
        # something different about the same run, the two would drift and
        # nobody would know which to believe.
        out = sweep("", survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="))

        self.assertIn("crates/core/src/timeline.rs:184", out)
        self.assertIn("signal, not a gate", out)

    def test_a_first_run_starts_the_history_rather_than_needing_one(self) -> None:
        # The tracking issue is opened by hand and its body says so; the first
        # sweep must not depend on that body already holding a table.
        out = sweep(None, caught("crates/core/src/keyframe.rs", 40))

        self.assertIn("## Catch rate over time", out)
        self.assertIn("| 2026-08-17 | `scorsese-core` |", out)

    def test_the_previous_readings_survive_and_the_new_one_goes_on_top(self) -> None:
        # The whole point of a schedule: one catch rate is a number, and the
        # question the sweep exists to answer is whether it is moving.
        old = (
            "<!-- mutation-sweep:history -->\n"
            "| Swept | Crate | Viable | Caught | Survived | Catch rate | Run |\n"
            "| --- | --- | --- | --- | --- | --- | --- |\n"
            "| 2026-08-10 | `scorsese-zimmer` | 900 | 700 | 200 | 78% | [log](x) |\n"
        )
        rows = [r for r in sweep(old, caught("a.rs", 40)).splitlines() if r.startswith("| 2026-")]

        self.assertEqual(len(rows), 2)
        self.assertIn("2026-08-17", rows[0])
        self.assertIn("2026-08-10", rows[1])

    def test_whatever_sits_above_the_history_marker_is_replaced(self) -> None:
        # The report is the *latest* reading and nothing else. Last month's
        # left sitting above a fresh table would be read as current.
        out = sweep(
            "<!-- mutation-sweep -->\nlast month's report, long since wrong.\n"
            "<!-- mutation-sweep:history -->\n"
            "| 2026-08-10 | `scorsese-zimmer` | 900 | 700 | 200 | 78% | [log](x) |\n",
            caught("crates/core/src/keyframe.rs", 40),
        )

        self.assertNotIn("long since wrong", out)
        self.assertIn("2026-08-10", out)

    def test_the_history_is_capped_so_the_body_cannot_outgrow_the_issue(self) -> None:
        # An issue body is capped at 65 536 characters, and a trend that
        # silently stopped updating for being too long is the worst of both.
        old = "<!-- mutation-sweep:history -->\n" + "".join(
            f"| 2026-01-01 | `scorsese-core` | 1 | 1 | 0 | 100% | — | {i}\n" for i in range(200)
        )
        out = sweep(old, caught("crates/core/src/keyframe.rs", 40))

        self.assertEqual(len([r for r in out.splitlines() if r.startswith("| 2026-")]), 120)

    def test_a_row_states_the_catch_rate_the_sweep_exists_to_track(self) -> None:
        out = sweep(
            "",
            caught("crates/core/src/keyframe.rs", 40),
            caught("crates/core/src/keyframe.rs", 41),
            caught("crates/core/src/keyframe.rs", 42),
            survivor("crates/core/src/timeline.rs", 184, "overlaps", "<="),
        )

        self.assertIn("| 4 | 3 | 1 | 75% |", out)


class ARunThatWasStopped(unittest.TestCase):
    """A truncated sweep reporting as a complete one is worse than no sweep."""

    STARTED = {"start_time": "2026-08-17T04:00:00Z"}

    def test_a_start_with_no_end_is_reported_as_unfinished(self) -> None:
        # cargo-mutants writes outcomes.json as it goes and stamps `end_time`
        # on the way out, so this is what a killed run leaves behind.
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(outcomes(total_mutants=3, caught=3, **self.STARTED)))
            result = run(path)

        self.assertIn("did not finish", result.stdout)

    def test_its_history_row_says_the_number_is_a_fraction_of_the_crate(self) -> None:
        out = sweep("", caught("crates/core/src/keyframe.rs", 40), **self.STARTED)

        self.assertIn("(cut short)", out)

    def test_a_run_that_finished_is_not_accused_of_stopping(self) -> None:
        out = sweep(
            "",
            caught("crates/core/src/keyframe.rs", 40),
            start_time="2026-08-17T04:00:00Z",
            end_time="2026-08-17T05:00:00Z",
        )

        self.assertNotIn("did not finish", out)
        self.assertNotIn("cut short", out)

    def test_a_run_with_no_times_at_all_is_not_a_truncated_run(self) -> None:
        # Every other fixture here writes outcomes with no timestamps, and
        # calling those truncated would invent a failure out of an absent
        # field. Only a start without an end means anything.
        self.assertNotIn("did not finish", render(caught("crates/core/src/keyframe.rs", 40)))


if __name__ == "__main__":
    unittest.main()
