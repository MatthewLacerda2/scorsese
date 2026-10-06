#!/usr/bin/env python3
"""#819: the watch says what changed on the board, and how to carry on.

Ported from MatthewLacerda2/rusty#909: one `watch: ` line per pull request that
opens, turns ready, goes back to draft or closes; the first listing says
nothing, so a relaunch does not replay the board. Added here: a listing GitHub
would not give is skipped rather than compared (the blip that announced every
pull request as new on 2026-10-05), each outcome is its own line, and the watch
ends on the command that resumes it.
"""

from __future__ import annotations

import importlib.util
import shlex
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("merge_queue_819", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
watch = queue.watch


def pr(number: int, draft=False, created="2026-10-06T12:00:00Z", *labels: str):
    return {
        "number": number,
        "isDraft": draft,
        "headRefOid": f"{number:040x}",
        "baseRefName": "main",
        "createdAt": created,
        "labels": [{"name": name} for name in labels],
    }


def board(*pulls, since=None):
    return watch.heads(list(pulls), since)


class Lines(unittest.TestCase):
    def test_the_first_listing_announces_nothing(self):
        self.assertEqual(watch.transitions(None, board(pr(1), pr(2, draft=True)), set()), [])

    def test_each_transition_has_its_line(self):
        before = board(pr(1, draft=True), pr(2), pr(3))
        after = board(pr(1), pr(2, draft=True), pr(4, draft=True), pr(5))
        self.assertEqual(watch.transitions(before, after, set()), [
            "watch: #1 turned ready.",
            "watch: #2 back to draft.",
            "watch: #3 closed, not merged by this watch.",
            "watch: #4 opened as a draft.",
            "watch: #5 opened as ready.",
        ])

    def test_one_the_watch_merged_is_not_reported_closed(self):
        lines = watch.transitions(board(pr(1), pr(2)), board(), {1})
        self.assertEqual(lines, ["watch: #2 closed, not merged by this watch."])

    def test_since_leaves_older_pull_requests_off_the_board(self):
        since = datetime(2026, 10, 6, 12, tzinfo=timezone.utc)
        old, new = pr(1, created="2026-10-05T09:00:00Z"), pr(2, created="2026-10-06T12:00:00Z")
        self.assertEqual(board(old, new, since=since), {2: False})

    def test_an_outcome_is_one_line(self):
        self.assertEqual(watch.outcome(7, "handed back", "it conflicts."), "watch: #7 handed back: it conflicts.")
        self.assertEqual(watch.outcome(0, "unreachable", "network."), "watch: unreachable: network.")


class Polling(unittest.TestCase):
    def test_a_failed_listing_is_skipped_never_compared(self):
        outage = queue.mergeable.Unreachable("network")
        answers = iter([[pr(1), pr(2, draft=True)], outage, None, [pr(1), pr(2)]])

        def lister():
            answer = next(answers)
            if isinstance(answer, Exception):
                raise answer
            return answer

        seen = watch.Board(lister=lister)
        said = [seen.poll() for _ in range(4)]
        # Neither the outage nor the garbled answer reads as everything
        # closing, and the next good listing is compared with the last good one.
        self.assertEqual(said, [[], [], [], ["watch: #2 turned ready."]])


class Resuming(unittest.TestCase):
    def test_the_resume_line_carries_every_argument(self):
        opts = queue.parse([
            "--watch", "--for", "50", "--deadline", "30", "--label", "go",
            "--since", "2026-10-06T16:00:00", "--no-merge",
        ])
        line = watch.resume(opts)
        self.assertTrue(line.startswith("watch: resume: python3 .github/scripts/merge-queue.py --watch"))
        # Reparsed, it is the same watch: what `resume` promises.
        command = shlex.split(line.removeprefix("watch: resume: "))
        again = queue.parse(command[2:])
        self.assertEqual(vars(again), vars(opts))

    def test_since_without_a_zone_is_utc(self):
        self.assertEqual(queue.since("2026-10-06T16:00:00").tzinfo, timezone.utc)

    def test_a_bad_since_is_refused(self):
        with mock.patch("sys.stderr"), self.assertRaises(SystemExit):
            queue.parse(["--watch", "--since", "yesterday"])


class Loop(unittest.TestCase):
    def test_the_watch_reports_the_board_and_each_outcome_as_it_goes(self):
        """The board is asked between CI polls too: #3 turns ready mid-wait."""
        clock, printed = [0.0], []
        boards = iter([
            [pr(1), pr(3, draft=True)],      # first: says nothing
            [pr(1), pr(3, draft=True)],      # mid-wait, before #3 flips
            [pr(1), pr(3)],                  # mid-wait: #3 turned ready
            [pr(3)],                         # #1 merged by this watch
        ])
        queued = iter([[pr(1, False, "2026-10-06T12:00:00Z", "queue")]])

        def gh(*args):
            if "--label" in args:
                return next(queued, [])
            return next(boards, [pr(3)])

        def take(repo, number, opts, heads=None, tick=None):
            tick()
            tick()
            clock[0] += 600
            return number, queue.MERGED, "CI passed on its head."

        opts = queue.parse(["--watch", "--for", "10"])
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch.object(queue, "take", side_effect=take), \
            mock.patch.object(watch, "clashing", return_value=set()), \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=lambda s: clock.__setitem__(0, clock[0] + s)), \
            mock.patch("builtins.print", side_effect=lambda *a, **k: printed.append(" ".join(map(str, a)))):
            self.assertEqual(queue.run_watch("o/r", opts), [
                (1, queue.MERGED, "CI passed on its head."),
            ])
        events = [line for line in printed if line.startswith("watch: ")]
        self.assertEqual(events, [
            "watch: #3 turned ready.",
            "watch: #1 merged: CI passed on its head.",
        ])

    def test_main_ends_on_the_resume_line(self):
        printed = []
        with mock.patch.object(queue.mergeable, "gh", return_value={"nameWithOwner": "o/r"}), \
            mock.patch.object(queue, "run_watch", return_value=[(4, queue.UNFINISHED, "still in line.")]), \
            mock.patch("builtins.print", side_effect=lambda *a, **k: printed.append(" ".join(map(str, a)))):
            status = queue.main(["--watch"])
        self.assertEqual(status, queue.UNFINISHED_STATUS)
        self.assertTrue(printed[-1].startswith("watch: resume: python3 "))

    def test_wait_for_asks_the_board_once_a_poll(self):
        ticks = []
        pull = {"headRefOid": "s" * 40, "isDraft": False}
        with mock.patch.object(queue, "look", return_value=pull), \
            mock.patch.object(queue, "evidence", return_value=([], {})), \
            mock.patch.object(queue, "progress", side_effect=[(queue.WAIT, ["waiting."]), (queue.GO, ["green."])]), \
            mock.patch.object(queue.time, "sleep"), \
            mock.patch("builtins.print"):
            state, _ = queue.wait_for("o/r", 1, "s" * 40, "s" * 40, 3600, 0, lambda: ticks.append(1))
        self.assertEqual((state, ticks), (queue.GO, [1]))


if __name__ == "__main__":
    unittest.main()
