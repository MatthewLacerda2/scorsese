"""#1757: a train's ride: green lands all on one run, red halves down to the culprit, a moved `main` retakes."""

from __future__ import annotations

import unittest

from .fakes import Fx, landing, merge, pr, queue, rails

LINE = [pr(1), pr(2), pr(3), pr(4)]


def go(fx: Fx, line=LINE, at=None):
    at = at or fx.yard()
    train = at.form(line)[1]
    return at, [(n, state) for n, state, _ in at.ride(train)]


class Riding(unittest.TestCase):
    def test_a_group_merges_as_one_run(self):
        fx = Fx(LINE)
        _, results = go(fx)
        self.assertEqual(results, [(n, queue.MERGED) for n in (1, 2, 3, 4)])
        self.assertEqual(fx.merged, [1, 2, 3, 4])  # in queue order, each pinned to its head
        self.assertEqual((fx.pushed, fx.waited), ([[1, 2, 3, 4]], ["main+h1+h2+h3+h4"]))
        self.assertEqual(fx.deleted, ["merge-train/main-1-2-3-4"])

    def test_a_red_group_is_split_down_to_its_culprit_and_the_rest_merge(self):
        # #4 is broken: the train is red, the half with it is red, and #4 alone
        # takes the one-at-a-time turn, which hands it back with its own run.
        fx = Fx(LINE, verdicts=(rails.RED, queue.GO, rails.RED))
        at, results = go(fx)
        self.assertEqual(results, [(n, queue.REQUEUED) for n in (1, 2, 3, 4)])
        self.assertEqual(at.waiting, [[(1, "h1"), (2, "h2")], [(3, "h3"), (4, "h4")]])
        self.assertEqual(at.red, {n: f"h{n}" for n in (1, 2, 3, 4)})  # alone until their heads move
        self.assertEqual(go(fx, at=at)[1], [(1, queue.MERGED), (2, queue.MERGED)])
        self.assertEqual(go(fx, LINE[2:], at)[1], [(3, queue.REQUEUED), (4, queue.REQUEUED)])
        self.assertEqual(at.waiting, [[(3, "h3")], [(4, "h4")]])
        self.assertEqual(at.form(LINE[2:])[:2], ([LINE[2]], None))
        self.assertEqual(at.form(LINE[3:])[:2], ([LINE[3]], None))
        self.assertEqual(fx.merged, [1, 2])

    def test_a_half_whose_head_moved_is_dropped(self):
        fx = Fx(LINE, verdicts=(rails.RED,))
        at, _ = go(fx)
        moved = [pr(1, head="new"), pr(2, head="new"), *LINE[2:]]
        self.assertEqual(at.form(moved)[1].numbers, [3, 4])

    def test_a_stalled_train_halves_marking_nobody_red(self):
        at, _ = go(Fx(LINE, verdicts=(rails.STALLED,)))
        self.assertEqual((at.red, len(at.waiting)), ({}, 2))

    def test_a_moved_main_retakes_the_group_never_merges_it(self):
        fx = Fx(LINE)
        fx.main_moved = 1
        at, results = go(fx)
        self.assertEqual((fx.merged, results), ([], [(n, queue.REQUEUED) for n in (1, 2, 3, 4)]))
        self.assertEqual(at.waiting, [[(n, f"h{n}") for n in (1, 2, 3, 4)]])
        fx.main_moved = 0
        self.assertEqual(go(fx, at=at)[1], [(n, queue.MERGED) for n in (1, 2, 3, 4)])
        self.assertEqual(len(fx.waited), 2)  # retested on the new `main`

    def test_main_not_holding_the_trains_tree_stops_the_watch(self):
        fx = Fx(LINE)
        fx.tree_override = "something else"
        self.assertEqual(go(fx)[1][-1], (0, queue.STOPPED))
        fx = Fx(LINE)
        real = fx.squash

        def squash(number, sha):
            fx.main_moved = 1 if number == 2 else fx.main_moved  # somebody merged between the calls
            return real(number, sha)

        fx.ns.squash = squash
        self.assertEqual(go(fx)[1][-1], (0, queue.STOPPED))
        self.assertEqual(queue.report.status([(0, queue.STOPPED, "x")]), queue.MISMATCH_STATUS)

    def test_a_member_that_moved_before_landing_lands_nobody(self):
        fx = Fx(LINE)
        fx.by[3] = pr(3, head="pushed")
        at, results = go(fx)
        self.assertEqual((fx.merged, results), ([], [(n, queue.REQUEUED) for n in (1, 2, 3, 4)]))
        self.assertEqual(at.waiting, [[(1, "h1"), (2, "h2"), (4, "h4")]])

    def test_a_refusal_mid_landing_hands_it_back_and_the_rest_go_back_in_line(self):
        for state, landed in ((queue.HANDED_BACK, [1]), (merge.UNKNOWN, [1]), (merge.BASE_MOVED, [1])):
            with self.subTest(state=state):
                fx = Fx(LINE)
                fx.squashes[2] = state
                _, results = go(fx)
                self.assertEqual(fx.merged, landed)
                self.assertEqual(results[0], (1, queue.MERGED))
                self.assertEqual(results[2:], [(3, queue.REQUEUED), (4, queue.REQUEUED)])
                self.assertNotIn(queue.STOPPED, [s for _, s in results])  # `main` holds #1's tree

    def test_a_refused_push_turns_trains_off_for_the_watch(self):
        fx = Fx(LINE)
        fx.push_refused = "could not push merge-train/main-1-2-3-4: denied"
        at, results = go(fx)
        self.assertEqual((results, fx.waited), ([(n, queue.REQUEUED) for n in (1, 2, 3, 4)], []))
        self.assertEqual(at.form(LINE)[:2], ([LINE[0]], None))
        self.assertEqual(len(fx.deleted), 1)

    def test_no_merge_stops_a_green_train_at_green(self):
        fx = Fx(LINE)
        fx.ns.no_merge = True
        self.assertEqual(go(fx)[1], [(n, queue.GREEN) for n in (1, 2, 3, 4)])
        self.assertEqual(fx.merged, [])

    def test_the_check_allows_one_more_for_a_merge_of_unknown_fate(self):
        train = Fx(LINE).yard().form(LINE)[1]
        self.assertIsNone(landing.check(train, 1, True, "main+h1+h2", 2))
        self.assertIn(landing.MISMATCH, landing.check(train, 1, False, "main+h1+h2", 2))


if __name__ == "__main__":
    unittest.main()
