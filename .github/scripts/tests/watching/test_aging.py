#!/usr/bin/env python3
"""No pull request waits forever in the watch's line (#1345).

Fewest-conflicts-first alone let #1292 lose every pick for about three hours on
2026-10-15. The ordering is pure and tested with the tally passed in; the loop
is tested with `gh`, `take` and the clock replaced, as in `test_loop.py`.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
watch = queue.watch


def pr(number: int, *labels: str, bot=False, mergeable="MERGEABLE"):
    return {
        "number": number,
        "isDraft": False,
        "headRefOid": f"{number:040x}",
        "baseRefName": "main",
        "createdAt": f"2026-10-15T10:{number % 60:02d}:00Z",
        "author": {"login": "app/dependabot" if bot else "someone"},
        "labels": [{"name": name} for name in labels],
        "closingIssuesReferences": [],
        "mergeable": mergeable,
    }


class Ordering(unittest.TestCase):
    def test_an_overdue_pull_request_goes_ahead_of_one_with_no_conflicts(self):
        pulls = [pr(1), pr(2), pr(3)]
        clashes = {frozenset((1, 3))}
        self.assertEqual(watch.pick(pulls, {}, {}, clashes, {1: 2})["number"], 2)
        self.assertEqual(watch.pick(pulls, {}, {}, clashes, {1: 3})["number"], 1)

    def test_dependabot_stays_last_however_long_it_waited(self):
        pulls = [pr(9, bot=True), pr(2)]
        self.assertEqual(watch.pick(pulls, {}, {}, set(), {9: 99})["number"], 2)

    def test_every_pass_counts_for_those_in_line_and_the_taken_one_starts_over(self):
        pulls = [pr(1), pr(2), pr(3, bot=True)]
        tally = watch.count_passes({1: 2}, pulls, {}, 2)
        self.assertEqual(tally, {1: 3})
        self.assertEqual(watch.count_passes(tally, pulls, {}, 1), {2: 1})

    def test_taking_an_overdue_one_is_said_with_its_count(self):
        pulls = [pr(1, "infrastructure"), pr(2, "feature")]
        line = watch.reordered(pulls, {}, {}, set(), {2: 3})
        self.assertEqual(
            line,
            "#2 goes before #1, which labels alone would take: #2 has been passed"
            " over 3 times, and 3 is as long as one waits.",
        )


class Watching(unittest.TestCase):
    def test_a_one_conflict_pull_request_is_taken_after_three_passes(self):
        # #1 conflicts with #2, which stays in line; a fresh conflict-free pull
        # request turns up before every pick, as on 2026-10-15.
        clock, taken = [0.0], []
        stream = iter(range(10, 30))
        line = [pr(1), pr(2)]

        def gh(*args):
            if "--label" not in args:
                return []  # the board (#819): `test_board.py`'s
            if not any(p["number"] >= 10 for p in line):
                line.append(pr(next(stream)))
            return list(line)

        def take(repo, number, opts, heads=None, tick=None):
            taken.append(number)
            line[:] = [p for p in line if p["number"] != number]
            clock[0] += 600
            return number, queue.MERGED, "why"

        def clashing(pulls, *_):
            present = {p["number"] for p in pulls}
            return {frozenset((1, 2))} if {1, 2} <= present else set()

        opts = queue.parse(["--watch", "--for", "60"])
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch.object(queue, "take", side_effect=take), \
            mock.patch.object(watch, "clashing", side_effect=clashing), \
            mock.patch.object(watch, "against_main", return_value={}), \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=lambda s: clock.__setitem__(0, clock[0] + s)), \
            mock.patch("builtins.print"):
            queue.run_watch("o/r", opts)
        self.assertEqual(taken[:4], [10, 11, 12, 1])


if __name__ == "__main__":
    unittest.main()
