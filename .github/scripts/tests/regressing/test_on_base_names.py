"""What a laid-over test meets on a real base, past what it imports (#1508).

The same throwaway repository as `test_on_base`: a test reaching the fix's new
function at run time is broken there, not failed; a fixture the head changed is
the one the base's run reads; a target past its time limit has failed.
"""

from __future__ import annotations

import unittest
from unittest import mock

from .test_on_base import HEADER, outcome, regression
from regression import BROKEN, FAILED, PASSED  # noqa: E402

BODY = HEADER + "import calc\n\n\nclass T(unittest.TestCase):\n    def test_it(self):\n"
LATE_API = BODY + "        self.assertEqual(calc.triple(2), 6)\n"
LATE_AND_WRONG = LATE_API + "\n    def test_double(self):\n        self.assertEqual(calc.double(5), 10)\n"
FIXTURE = "fixtures/want.txt"
READS = BODY + "        want = (Path(__file__).resolve().parents[1] / 'fixtures/want.txt').read_text()\n" \
    "        self.assertEqual(calc.double(5), int(want))\n"
SLEEPS = BODY + "        import time; time.sleep(30)\n"
NONE_CRASH = BODY + "        self.assertIsNone(None.real)\n"
SUBTEST = BODY + "        for x in (5,):\n            with self.subTest(x=x):\n" \
    "                self.assertEqual(calc.double(x), 10)\n"


class Names(unittest.TestCase):
    def outcome(self, test: str, base: dict | None = None, head: dict | None = None) -> str:
        return outcome(self, test, base, head)

    def test_a_new_function_reached_at_run_time_is_broken_on_the_base(self) -> None:
        self.assertEqual(self.outcome(LATE_API), BROKEN)

    def test_an_attribute_missing_on_an_object_is_the_bug_so_it_fails(self) -> None:
        self.assertEqual(self.outcome(NONE_CRASH), FAILED)

    def test_a_failing_subtest_fails(self) -> None:
        self.assertEqual(self.outcome(SUBTEST), FAILED)

    def test_a_missing_name_beside_a_real_failure_still_fails(self) -> None:
        self.assertEqual(self.outcome(LATE_AND_WRONG), FAILED)

    def test_the_heads_fixture_is_the_one_the_base_reads(self) -> None:
        # The base's own fixture agrees with its bug; only the head's shows it.
        self.assertEqual(self.outcome(READS, {FIXTURE: "7"}, {FIXTURE: "10"}), FAILED)
        self.assertEqual(self.outcome(READS, {FIXTURE: "7"}), PASSED)

    def test_a_target_past_its_limit_has_failed(self) -> None:
        with mock.patch.object(regression, "TIMEOUT", 2):
            self.assertEqual(self.outcome(SLEEPS), FAILED)
