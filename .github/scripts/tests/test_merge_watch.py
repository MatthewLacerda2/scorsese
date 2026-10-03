#!/usr/bin/env python3
"""The watch takes what it is cleared to take, in the order CLAUDE.md gives (#690).

Two halves. `merge-watch.py` chooses, and is pure: priority, age, the go-ahead,
and the memory that keeps a hand-back from being retried every poll. The loop
in `merge-queue.py` is tested with `gh` replaced at the one call it makes per
poll, `take` replaced by the outcomes a real branch would have, and a clock
that only moves when the loop sleeps — so a two-hour watch runs in
milliseconds and no test touches a network.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


queue = load("merge_queue", "merge-queue.py")
watch = queue.watch


def pr(number: int, *labels: str, closes=(), draft=False, head=None, created=None, base="main"):
    return {
        "number": number,
        "isDraft": draft,
        "headRefOid": head or f"{number:040x}",
        "baseRefName": base,
        "createdAt": created or f"2026-10-03T10:{number % 60:02d}:00Z",
        "labels": [{"name": name} for name in labels],
        "closingIssuesReferences": [{"number": n} for n in closes],
    }


class Choosing(unittest.TestCase):
    def test_infrastructure_goes_before_an_older_feature(self):
        pulls = [pr(1, "feature"), pr(2, "infrastructure")]
        self.assertEqual(watch.pick(pulls, {}, {})["number"], 2)

    def test_the_whole_priority_order_is_claude_mds(self):
        names = ["feature", "foundation", "bug", "architecture", "infrastructure"]
        ranks = [watch.rank([name]) for name in names]
        self.assertEqual(ranks, sorted(ranks, reverse=True))
        self.assertEqual(len(set(ranks)), len(ranks))

    def test_documentation_never_waits_its_turn(self):
        self.assertEqual(watch.rank(["documentation"]), watch.rank(["infrastructure"]))

    def test_an_unlabelled_pull_request_goes_last_not_never(self):
        pulls = [pr(1), pr(2, "feature")]
        self.assertEqual([p["number"] for p in watch.ordered(pulls, {})], [2, 1])

    def test_within_a_rank_the_oldest_goes_first(self):
        old = pr(9, "bug", created="2026-10-01T00:00:00Z")
        new = pr(3, "bug", created="2026-10-03T00:00:00Z")
        self.assertEqual(watch.pick([new, old], {}, {})["number"], 9)

    def test_the_closed_issue_carries_the_type_when_the_pull_request_does_not(self):
        # The usual case here: the issue is labelled, its pull request is not.
        pulls = [pr(1, "queue", closes=[50]), pr(2, "queue", closes=[60])]
        issues = {50: ["feature"], 60: ["infrastructure"]}
        self.assertEqual(watch.pick(pulls, {}, issues)["number"], 2)

    def test_a_draft_is_not_taken(self):
        self.assertIsNone(watch.pick([pr(1, "bug", draft=True)], {}, {}))

    def test_a_pull_request_against_another_base_is_not_taken(self):
        self.assertIsNone(watch.pick([pr(1, base="stacked")], {}, {}))

    def test_a_hand_back_is_skipped_until_its_head_moves(self):
        handed = pr(1, "infrastructure", head="a" * 40)
        self.assertIsNone(watch.pick([handed], {1: "a" * 40}, {}))
        pushed = pr(1, "infrastructure", head="b" * 40)
        self.assertEqual(watch.pick([pushed], {1: "a" * 40}, {})["number"], 1)

    def test_what_is_in_line_excludes_what_was_already_dealt_with(self):
        pulls = [pr(1), pr(2, head="c" * 40), pr(3, draft=True)]
        self.assertEqual(watch.in_line(pulls, {2: "c" * 40}), [1])


class Budget(unittest.TestCase):
    def test_the_defaults_fit_the_cap(self):
        self.assertIsNone(watch.budget_error(watch.FOR_MINUTES, queue.DEADLINE_MINUTES))
        self.assertLess(watch.CAP_MINUTES, 120)

    def test_a_watch_longer_than_the_harness_allows_is_refused_up_front(self):
        problem = watch.budget_error(100, 40)
        self.assertIn("two hours", problem)

    def test_the_limits_are_checked_when_the_flags_are_parsed(self):
        with mock.patch("sys.stderr"), self.assertRaises(SystemExit):
            queue.parse(["--watch", "--for", "90", "--deadline", "40"])

    def test_a_watch_and_a_list_together_are_refused(self):
        with mock.patch("sys.stderr"), self.assertRaises(SystemExit):
            queue.parse(["--watch", "486"])

    def test_neither_a_watch_nor_a_list_is_refused(self):
        with mock.patch("sys.stderr"), self.assertRaises(SystemExit):
            queue.parse([])
