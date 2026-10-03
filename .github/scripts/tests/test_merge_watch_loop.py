#!/usr/bin/env python3
"""The watch loop in `merge-queue.py`, with GitHub and the clock replaced (#690).

`gh` is replaced at `mergeable.gh` — the one door every question goes through —
and answers each `pr list` with the next listing in a script, so a test reads
as *what GitHub showed, poll by poll*. `take` is replaced by the outcome a real
branch would have had: its per-branch steps are `test_merge_queue.py`'s. Time
moves only when the loop sleeps, so `--for` is honoured without waiting it out.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)


def pr(number: int, *labels: str, head=None, closes=()):
    return {
        "number": number,
        "isDraft": False,
        "headRefOid": head or f"{number:040x}",
        "baseRefName": "main",
        "createdAt": f"2026-10-03T10:{number % 60:02d}:00Z",
        "labels": [{"name": name} for name in labels],
        "closingIssuesReferences": [{"number": n} for n in closes],
    }


class Watching(unittest.TestCase):
    def watch(self, listings, outcomes, issues=None, extra=(), cost=600):
        """Run a watch over `listings` (one per poll; the last repeats).

        `outcomes` maps a number to `(state, head_after)`. Returns the results
        and the numbers taken, in order.
        """
        clock = [0.0]
        polls = iter(listings)
        last = [listings[-1]]
        taken = []

        def gh(*args):
            if args[:2] == ("pr", "list"):
                last[0] = next(polls, last[0])
                return last[0]
            if args[:2] == ("issue", "view"):
                return {"labels": [{"name": n} for n in (issues or {}).get(int(args[2]), [])]}
            raise AssertionError(f"unexpected gh {args}")

        def take(repo, number, opts, heads=None):
            taken.append(number)
            state, head = outcomes[number]
            if heads is not None:
                heads[number] = head
            clock[0] += cost  # a CI run, and the rest of one branch
            return number, state, "why"

        def sleep(seconds):
            clock[0] += seconds

        opts = queue.parse(["--watch", "--for", "60", *extra])
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch.object(queue, "take", side_effect=take), \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=sleep), \
            mock.patch("builtins.print"):
            return queue.run_watch("o/r", opts), taken

    def test_it_takes_in_priority_order_as_pull_requests_appear(self):
        a, b, c = pr(1, "feature"), pr(2, "infrastructure"), pr(3, "bug")
        listings = [[], [a, b], [a, c], [a], []]
        outcomes = {n: (queue.MERGED, "x") for n in (1, 2, 3)}
        results, taken = self.watch(listings, outcomes)
        # #2 outranks #1 at the first sight of both; #3 arrives and outranks #1.
        self.assertEqual(taken, [2, 3, 1])
        self.assertTrue(all(state == queue.MERGED for _, state, _ in results))

    def test_a_hand_back_is_not_retried_until_somebody_pushes(self):
        before, after = pr(5, head="a" * 40), pr(5, head="b" * 40)
        listings = [[before], [before], [before], [after], []]
        outcomes = {5: (queue.HANDED_BACK, "a" * 40)}
        _, taken = self.watch(listings, outcomes)
        self.assertEqual(taken, [5, 5])

    def test_the_head_the_queue_pushed_is_the_one_remembered(self):
        # Red after a rebase: the branch now sits on the queue's own push, and
        # seeing that head is not a reason to take it again.
        listings = [[pr(6, head="a" * 40)], [pr(6, head="p" * 40)]]
        _, taken = self.watch(listings, {6: (queue.HANDED_BACK, "p" * 40)})
        self.assertEqual(taken, [6])

    def test_what_is_still_in_line_at_the_end_is_unfinished_not_handed_back(self):
        # One branch outlasts the hour; #8 was cleared the whole time.
        listings = [[pr(7, "infrastructure"), pr(8)], [pr(8)]]
        results, taken = self.watch(listings, {7: (queue.MERGED, "x")}, cost=3700)
        self.assertEqual(taken, [7])
        self.assertIn((8, queue.UNFINISHED, "still in line when the watch ended."), results)
        self.assertEqual(queue.status(results), queue.UNFINISHED_STATUS)

    def test_a_merge_the_listing_has_not_caught_up_with_is_not_taken_twice(self):
        listings = [[pr(4)], [pr(4)], []]
        results, taken = self.watch(listings, {4: (queue.MERGED, f"{4:040x}")})
        self.assertEqual(taken, [4])
        self.assertEqual(queue.status(results), 0)

    def test_an_idle_watch_ends_on_time_with_nothing_to_report(self):
        results, taken = self.watch([[]], {})
        self.assertEqual((results, taken), ([], []))

    def test_the_closed_issue_decides_the_order(self):
        one, two = pr(1, "queue", closes=[10]), pr(2, "queue", closes=[20])
        listings = [[one, two], [one], []]
        outcomes = {1: (queue.MERGED, "x"), 2: (queue.MERGED, "x")}
        _, taken = self.watch(listings, outcomes, issues={10: ["feature"], 20: ["bug"]})
        self.assertEqual(taken, [2, 1])

    def test_github_going_quiet_stops_the_watch_with_its_own_status(self):
        outage = queue.mergeable.Unreachable("network: GitHub unreachable")
        opts = queue.parse(["--watch"])
        with mock.patch.object(queue.mergeable, "gh", side_effect=outage), \
            mock.patch("builtins.print"):
            results = queue.run_watch("o/r", opts)
        self.assertEqual(queue.status(results), queue.mergeable.UNREACHABLE_STATUS)
        self.assertIn("watch: unreachable", queue.summary(results)[0])


class Ending(unittest.TestCase):
    def test_a_run_still_out_at_the_deadline_is_unfinished(self):
        pull = {"headRefOid": "s" * 40, "isDraft": False}
        live = [{"name": "CI", "head_sha": "s" * 40, "id": 1, "status": "in_progress"}]
        with mock.patch.object(queue, "look", return_value=pull), \
            mock.patch.object(queue, "evidence", return_value=(live, {1: []})), \
            mock.patch.object(queue.time, "sleep"), \
            mock.patch("builtins.print"):
            state, lines = queue.wait_for("o/r", 1, "s" * 40, "s" * 40, -1, 0)
        self.assertEqual(state, queue.LATE)
        self.assertIn("Nothing is red", " ".join(lines))

    def test_take_reports_the_head_it_pushed_and_a_late_run_as_unfinished(self):
        pull = {"state": "OPEN", "isDraft": False, "headRefName": "b", "headRefOid": "o" * 40}
        late = (queue.LATE, ["still waiting on ppppppp after 40 minutes."])
        heads = {}
        with mock.patch.object(queue, "look", return_value=pull), \
            mock.patch.object(queue, "git"), \
            mock.patch.object(queue, "advance", return_value=("p" * 40, [])), \
            mock.patch.object(queue, "wait_for", return_value=late), \
            mock.patch("builtins.print"):
            _, state, _ = queue.take("o/r", 1, queue.parse(["1"]), heads)
        self.assertEqual(state, queue.UNFINISHED)
        self.assertEqual(heads, {1: "p" * 40})

    def test_unfinished_has_its_own_status_below_a_hand_back(self):
        unfinished = [(1, queue.MERGED, ""), (2, queue.UNFINISHED, "")]
        self.assertEqual(queue.status(unfinished), queue.UNFINISHED_STATUS)
        self.assertEqual(queue.status(unfinished + [(3, queue.HANDED_BACK, "")]), 1)
        self.assertEqual(queue.status([(1, queue.MERGED, "")]), 0)
        self.assertIn("Unfinished is not red", "\n".join(queue.summary(unfinished)))

    def test_the_listing_asks_for_ready_labelled_pull_requests(self):
        done = subprocess.CompletedProcess([], 0, json.dumps([]), "")
        with mock.patch.object(queue.mergeable.subprocess, "run", return_value=done) as ran:
            queue.watch.waiting("queue")
        command = ran.call_args.args[0]
        self.assertEqual(command[:3], ["gh", "pr", "list"])
        self.assertIn("queue", command)
        self.assertIn("closingIssuesReferences", command[-1])


if __name__ == "__main__":
    unittest.main()
