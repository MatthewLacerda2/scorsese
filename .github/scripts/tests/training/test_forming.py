"""#1757: who rides a train, who waits for the next, who goes alone ([`yard.Yard.form`], [`group.form`])."""

from __future__ import annotations

import unittest

from .fakes import MIGRATIONS, Fx, formed, group, pr


class Forming(unittest.TestCase):
    def test_the_front_of_the_line_rides_together_up_to_the_cap(self):
        line = [pr(n) for n in range(1, 8)]
        self.assertEqual(formed(Fx(line), line)[1], [1, 2, 3, 4])
        self.assertEqual(group.CAP, 4)

    def test_a_member_conflicting_inside_the_group_waits_for_the_next_and_keeps_its_place(self):
        line = [pr(1), pr(2), pr(3)]
        members, train, notes = formed(Fx(line, clashing={"h2"}), line)
        self.assertEqual((members, train), ([1, 3], [1, 3]))
        self.assertIn("#2 waits for the next train: it does not merge cleanly onto the train so far (#1)", notes[0])
        # The next round's line is the watch's, unchanged: #2 leads it.
        self.assertEqual(formed(Fx([pr(2), pr(4)]), [pr(2), pr(4)])[1], [2, 4])

    def test_two_members_taking_the_same_migration_number_never_share_a_group(self):
        line = [pr(1), pr(2), pr(3)]
        adds = {"h1": [f"{MIGRATIONS}/0042_one.sql"], "h2": [f"{MIGRATIONS}/0042_two.sql"], "h3": [f"{MIGRATIONS}/0043_three.sql"]}
        members, train, notes = formed(Fx(line, adds=adds), line)
        self.assertEqual(train, [1, 3])
        self.assertIn("#2 waits for the next train: Migration number already taken", notes[0])
        self.assertIn("0042_two.sql", notes[0])

    def test_a_first_that_takes_a_number_main_holds_goes_alone_to_be_handed_back(self):
        line = [pr(1), pr(2)]
        adds = {"main": [f"{MIGRATIONS}/0042_main.sql"], "h1": [f"{MIGRATIONS}/0042_one.sql"]}
        self.assertEqual(formed(Fx(line, adds=adds), line)[:2], ([1], None))

    def test_what_must_go_alone_waits_behind_and_goes_alone_first(self):
        for path in (".github/scripts/merge_queue/yard.py", ".github/scripts/ci-ok.py", ".github/workflows/ci.yml", "Cargo.lock", "Makefile", "docs/testing.md"):
            with self.subTest(path=path):
                line = [pr(1), pr(2), pr(3)]
                self.assertEqual(formed(Fx(line, files={1: [path]}), line)[:2], ([1], None))
                members, train, notes = formed(Fx(line, files={2: [path]}), line)
                self.assertEqual(train, [1, 3])
                self.assertIn("#2 waits for a train of its own: it", notes[0])

    def test_a_page_the_code_reads_rides(self):
        self.assertIsNone(group.alone(["docs/project-format.md"]))
        self.assertIsNone(group.alone(["crates/core/src/lib.rs", "README.md"]))

    def test_every_per_pull_request_judgement_stays_one(self):
        line = [pr(1), pr(2), pr(3)]
        refused = {2: "its own head is not cleared: a CI run for h2 concluded failure"}
        members, train, notes = formed(Fx(line, refused=refused), line)
        self.assertEqual(train, [1, 3])
        self.assertIn("concluded failure", notes[0])
        self.assertEqual(formed(Fx(line, refused={1: "a `planning` issue"}), line)[:2], ([1], None))

    def test_dependabot_and_a_train_of_one_take_the_turn_alone(self):
        bot = {**pr(1), "author": {"login": "dependabot[bot]"}}
        self.assertEqual(formed(Fx([bot, pr(2)]), [bot, pr(2)])[:2], ([1], None))
        line = [pr(1), pr(2)]
        self.assertEqual(formed(Fx(line, clashing={"h1"}), line)[:2], ([1], None))  # conflicts with `main`
        self.assertEqual(formed(Fx(line, clashing={"h2"}), line)[:2], ([1], None))

    def test_a_head_red_in_a_train_goes_alone_until_it_moves(self):
        line = [pr(1), pr(2)]
        fx = Fx(line)
        at = fx.yard()
        at.red[1] = "h1"
        self.assertIsNone(formed(fx, line, at)[1])
        self.assertEqual(formed(fx, [pr(1, head="new"), pr(2)], at)[1], [1, 2])

    def test_trains_turned_off_take_one_at_a_time(self):
        line = [pr(1), pr(2)]
        fx = Fx(line)
        at = fx.yard()
        at.off = "could not push"
        self.assertEqual(formed(fx, line, at), ([1], None, []))


if __name__ == "__main__":
    unittest.main()
