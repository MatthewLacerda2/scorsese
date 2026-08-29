#!/usr/bin/env python3
"""The mutation signal renders a run that had nothing to do.

Why this file exists at all: #40's rule that a gate nobody tests can stop
gating without saying so, applied to a signal. The failure it is written
against is a real one — `make mutants` died with a `FileNotFoundError`
traceback, and CI reported "the run did not produce a report", on every branch
that changes only test files. Both said the tool had broken when it had not.

The script is exercised the way its callers call it: as a subprocess, on its
argv contract, asserting on what lands on stdout and on the exit status. A
test that imported the module and called `headline` directly would have passed
throughout the bug, because the bug was never in `headline`.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "mutants-summary.py"


def run(argument: Path | str, *rest: str) -> subprocess.CompletedProcess[str]:
    """The script, invoked as `make` and the workflow invoke it."""
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(argument), *rest],
        capture_output=True,
        text=True,
        check=False,
    )


def outcomes(**counts: int) -> dict:
    """A run's `outcomes.json`, with the totals a caller wants to state."""
    return {
        "total_mutants": 0,
        "missed": 0,
        "caught": 0,
        "timeout": 0,
        "unviable": 0,
        "outcomes": [],
        **counts,
    }


def mutant(summary: str, file: str, line: int, function: str, replacement: str) -> dict:
    """One mutant and what became of it, shaped as cargo-mutants records it."""
    return {
        "summary": summary,
        "scenario": {
            "Mutant": {
                "file": file,
                "span": {"start": {"line": line}},
                "function": {"function_name": function},
                "replacement": replacement,
            }
        },
    }


def survivor(file: str, line: int, function: str, replacement: str) -> dict:
    """A mutant no test objected to."""
    return mutant("MissedMutant", file, line, function, replacement)


def caught(file: str, line: int) -> dict:
    """A mutant of the same file that a local test did object to."""
    return mutant("CaughtMutant", file, line, "evaluate", "0.0")


def render(*outcome_list: dict, **counts: int) -> str:
    """The report for a run made of these outcomes."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "outcomes.json"
        missed = sum(1 for o in outcome_list if o["summary"] == "MissedMutant")
        path.write_text(
            json.dumps(
                outcomes(
                    total_mutants=len(outcome_list),
                    missed=missed,
                    caught=len(outcome_list) - missed,
                    outcomes=list(outcome_list),
                    **counts,
                )
            )
        )
        result = run(path)
    assert result.returncode == 0, result.stderr
    return result.stdout


class NothingToRun(unittest.TestCase):
    """The case the script used to die on."""

    def test_a_missing_outcomes_file_is_reported_rather_than_raised(self) -> None:
        # cargo-mutants writes no outcomes.json when the diff it is given holds
        # no mutable lines. A branch that only adds tests is exactly that, and
        # it is the shape of an ordinary pull request — not of a broken tool.
        with tempfile.TemporaryDirectory() as tmp:
            result = run(Path(tmp) / "outcomes.json")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("No mutants to run", result.stdout)
        self.assertNotIn("Traceback", result.stderr)

    def test_nothing_to_run_says_why_rather_than_only_that(self) -> None:
        # The reader has to be able to tell "nothing was in scope" from "the
        # comparison went wrong", which is the whole reason the scope note is
        # never omitted.
        with tempfile.TemporaryDirectory() as tmp:
            result = run(Path(tmp) / "outcomes.json", "compared against `main`.")

        self.assertIn("mutated surface", result.stdout)
        self.assertIn(".cargo/mutants.toml", result.stdout)
        self.assertIn("compared against `main`.", result.stdout)

    def test_an_empty_run_that_did_write_a_file_reads_the_same_way(self) -> None:
        # Whether cargo-mutants wrote a file with no mutants in it or wrote no
        # file at all is an implementation detail of the tool. The report must
        # not depend on which happened.
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(outcomes()))
            written = run(path)
            absent = run(Path(tmp) / "missing.json")

        self.assertEqual(written.stdout, absent.stdout)


class ARunWithResults(unittest.TestCase):
    """The ordinary path, so the fix above cannot swallow a real report."""

    def test_survivors_are_listed_with_the_mutation_spelled_out(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(
                json.dumps(
                    outcomes(
                        total_mutants=2,
                        missed=1,
                        caught=1,
                        outcomes=[survivor("crates/core/src/timeline.rs", 184, "overlaps", "<=")],
                    )
                )
            )
            result = run(path)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("1 of 2 mutations survived", result.stdout)
        self.assertIn("crates/core/src/timeline.rs:184", result.stdout)
        self.assertIn("`overlaps`", result.stdout)
        self.assertIn("<=", result.stdout)

    def test_a_clean_run_says_so_rather_than_saying_nothing_ran(self) -> None:
        # The distinction the bug erased: "every mutation was caught" and
        # "there was nothing to mutate" are different results.
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(outcomes(total_mutants=3, caught=3)))
            result = run(path)

        self.assertIn("All 3 mutations were caught", result.stdout)
        self.assertNotIn("No mutants to run", result.stdout)


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


def sweep(previous: str | None, *outcome_list: dict, **counts: int) -> str:
    """The sweep's whole issue body, given the body it is replacing."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "outcomes.json"
        missed = sum(1 for o in outcome_list if o["summary"] == "MissedMutant")
        path.write_text(
            json.dumps(
                outcomes(
                    total_mutants=len(outcome_list),
                    missed=missed,
                    caught=len(outcome_list) - missed,
                    outcomes=list(outcome_list),
                    **counts,
                )
            )
        )
        body = Path(tmp) / "current.md"
        if previous is not None:
            body.write_text(previous)
        result = run(
            path,
            "Crate `scorsese-core`.",
            "--issue-body",
            str(body),
            "--subject",
            "scorsese-core",
            "--run-url",
            "https://example.invalid/run/1",
            "--now",
            "2026-08-17",
        )
    assert result.returncode == 0, result.stderr
    return result.stdout


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


def report(*outcome_list: dict, scope: str = "", args: tuple[str, ...] = (), **counts: int) -> str:
    """The report for this run, rendered with whatever extra argv is passed."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "outcomes.json"
        missed = sum(1 for o in outcome_list if o["summary"] == "MissedMutant")
        path.write_text(
            json.dumps(
                outcomes(
                    total_mutants=len(outcome_list),
                    missed=missed,
                    caught=len(outcome_list) - missed,
                    outcomes=list(outcome_list),
                    **counts,
                )
            )
        )
        result = run(path, scope, *args)
    assert result.returncode == 0, result.stderr
    return result.stdout


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

        self.assertIn("137 of 233 mutations in this diff were not measured", out)
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

        self.assertIn("1 of 2 mutations in this diff were not measured", out)

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


class WhatEarnsAComment(unittest.TestCase):
    """The marker the `mutants` job greps for, decided here rather than there.

    It used to be a grep for a heading, which cannot see the one report this
    change adds: a gap has no survivors and no timeouts, so the message #399
    exists to deliver would never have reached the pull request.
    """

    MARKER = "<!-- mutation-signal:notable -->"

    def test_a_survivor_earns_one(self) -> None:
        self.assertIn(self.MARKER, report(survivor("a.rs", 1, "f", "()")))

    def test_a_timeout_earns_one(self) -> None:
        timed_out = mutant("Timeout", "crates/core/src/keyframe.rs", 61, "evaluate", "0.0")
        self.assertIn(self.MARKER, report(timed_out, timeout=1))

    def test_a_gap_earns_one_even_with_nothing_else_to_show(self) -> None:
        out = report(caught("crates/core/src/keyframe.rs", 40), args=("--in-scope", "50"))

        self.assertNotIn("### Survivors", out)
        self.assertNotIn("### Timed out", out)
        self.assertIn(self.MARKER, out)

    def test_a_clean_complete_run_earns_nothing(self) -> None:
        # Silence is the right output here, and it is what keeps the comment
        # from being noise on every pull request that has nothing wrong.
        out = report(caught("crates/core/src/keyframe.rs", 40), args=("--in-scope", "1"))

        self.assertNotIn(self.MARKER, out)

    def test_nothing_to_run_earns_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run(Path(tmp) / "outcomes.json")

        self.assertNotIn(self.MARKER, result.stdout)


class TheArgvContract(unittest.TestCase):
    def test_no_arguments_is_a_usage_error_and_not_a_report(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT)], capture_output=True, text=True, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("usage:", result.stderr)


if __name__ == "__main__":
    unittest.main()
