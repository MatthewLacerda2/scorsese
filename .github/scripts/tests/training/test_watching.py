"""#1757: the watch with trains: a group taken on one pick, a member back in line taken again, a stop.

GitHub's listing is faked at `mergeable.gh` as in `watching/test_loop.py`, and
the yard runs on [`fakes.Fx`]; `take`, the one-at-a-time turn, is replaced by
its outcome.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from .fakes import Fx, pr, queue, rails


def rest(number: int) -> dict:
    return {
        "number": number, "state": "open", "draft": False, "user": {"login": "x"},
        "head": {"sha": f"h{number}", "ref": f"b{number}"}, "base": {"ref": "main"},
        "created_at": f"2026-11-04T10:{number:02d}:00Z", "labels": [{"name": "queue"}], "body": "",
    }


class Watching(unittest.TestCase):
    def watch(self, fx: Fx, numbers=(1, 2, 3), kept=None):
        clock, taken, printed = [0.0], [], []

        def gh(api, path):
            if path.startswith("repos/o/r/pulls?state=open&"):
                return [rest(n) for n in numbers if n not in fx.merged and n not in taken]
            raise AssertionError(f"unexpected gh {api} {path}")

        def take(repo, number, opts, heads=None, tick=None):
            taken.append(number)
            clock[0] += 600
            return number, queue.MERGED, "merged alone."

        def sleep(seconds):
            clock[0] += seconds

        opts = queue.parse(["--watch", "--for", "30"])
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch.object(queue.watch, "everything", return_value=[]), \
            mock.patch.object(queue, "take", side_effect=take), \
            mock.patch.object(queue.watch, "clashing", return_value=set()), \
            mock.patch.object(queue.watch, "against_main", return_value={}), \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=sleep), \
            mock.patch("builtins.print", side_effect=lambda *a, **k: printed.append(" ".join(map(str, a)))):
            results = queue.run_watch("o/r", opts, kept, trains=fx.yard())
        return results, taken, [line for line in printed if line.startswith("watch: ")]

    def test_a_group_is_one_pick_and_its_members_are_merged_by_it(self):
        fx = Fx([pr(n) for n in (1, 2, 3)])
        results, taken, said = self.watch(fx)
        self.assertEqual([(n, s) for n, s, _ in results], [(1, queue.MERGED), (2, queue.MERGED), (3, queue.MERGED)])
        self.assertEqual((fx.merged, taken, fx.pushed), ([1, 2, 3], [], [[1, 2, 3]]))
        self.assertIn("watch: taking #1, #2, #3 together.", said)

    def test_a_trains_members_are_not_counted_as_passed_over(self):
        with tempfile.TemporaryDirectory() as tmp:
            kept = Path(tmp) / "tally.json"
            self.watch(Fx([pr(n) for n in (1, 2, 3)]), kept=kept)
            self.assertEqual(queue.tally.load(kept), {})

    def test_a_member_back_in_line_is_taken_again_and_a_half_of_one_alone(self):
        fx = Fx([pr(n) for n in (1, 2, 3)], verdicts=(rails.RED, rails.RED))
        results, taken, said = self.watch(fx)
        # Red: halved into [#1] and [#2, #3]. [#1] goes alone (taken); [#2, #3] is red
        # again, so #2 and #3 each go alone, as today's turn.
        self.assertEqual(taken, [1, 2, 3])
        self.assertEqual(fx.pushed, [[1, 2, 3], [2, 3]])
        self.assertIn("watch: #2 goes back in line: its train was red; its half is tested next.", said)
        self.assertEqual(sorted(n for n, s, _ in results if s == queue.MERGED), [1, 2, 3])

    def test_main_not_the_trains_tree_stops_the_watch_with_its_own_status(self):
        fx = Fx([pr(n) for n in (1, 2)])
        fx.tree_override = "elsewhere"
        results, taken, said = self.watch(fx, (1, 2))
        self.assertEqual(results[-1][:2], (0, queue.STOPPED))
        self.assertEqual(queue.report.status(results), queue.MISMATCH_STATUS)
        self.assertTrue(any(line.startswith("watch: STOPPED: main's tree is not the train's") for line in fx.said))
        self.assertIn("watch: stopped: main's tree is not the train's", " ".join(said))


if __name__ == "__main__":
    unittest.main()
