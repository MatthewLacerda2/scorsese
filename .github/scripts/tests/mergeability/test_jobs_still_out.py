"""A run GitHub marks completed while a job is still out has not passed (#1764).

`fixtures/run-completed-jobs-still-out.json` is run 41861194585 on #1761 as the
queue polled it during the 2026-11-04 Actions outage: the run read `completed`
/ `success` while `fmt + clippy + test` was still `in_progress` (every step
done) and `CI ok` was `queued`. The queue called it passed and asked GitHub to
merge; only `main`'s ruleset refused. Recorded from the run's settled answer
with those two jobs set back to what the queue saw, steps trimmed to the first
and last. All three readers (the one-shot `judge`, the queue's poll, the
train's tip) are asked here, because a train's members are not covered by the
ruleset.
"""

from __future__ import annotations

import copy
import importlib.util
import json
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPTS = HERE.parents[1]
FIXTURE = json.loads((HERE.parent / "fixtures" / "run-completed-jobs-still-out.json").read_text())

_spec = importlib.util.spec_from_file_location("merge_queue_1764", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
mergeable, verdict, rails = queue.mergeable, queue.verdict, queue.yard.rails

RUN = FIXTURE["run"]
PULL = {"isDraft": False, "headRefOid": RUN["head_sha"], "number": 1761, "state": "OPEN",
        "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN"}


def jobs(**changed: dict) -> dict[int, list[dict]]:
    """The recorded jobs, with the job named by each key's fields changed."""
    listed = copy.deepcopy(FIXTURE["jobs"])
    for job in listed:
        job.update(changed.get(job["name"], {}))
    return {RUN["id"]: listed}


DONE = {"status": "completed", "conclusion": "success"}
SETTLED = jobs(**{"fmt + clippy + test": DONE, "CI ok": DONE})


def judged(evidence: dict[int, list[dict]]) -> tuple[bool, list[str]]:
    return mergeable.judge(PULL, [RUN], evidence)


def polled(evidence: dict[int, list[dict]], waited: float = 60) -> str:
    return verdict.progress(PULL, [RUN], evidence, waited)[0]


def tip(evidence: dict[int, list[dict]], waited: float = 60) -> str:
    return rails.tip_state([RUN], evidence, waited, frozenset())[0]


class TheRecordedRun(unittest.TestCase):
    def test_it_is_unfinished_though_github_calls_it_completed(self):
        self.assertEqual(mergeable.unfinished([RUN], jobs()), [RUN])
        self.assertEqual(mergeable.unfinished([RUN], SETTLED), [])

    def test_judge_refuses_it_as_in_flight(self):
        ok, lines = judged(jobs())
        self.assertFalse(ok)
        self.assertIn("`fmt + clippy + test` is in_progress", lines[0])

    def test_the_queue_waits_on_it(self):
        self.assertEqual(polled(jobs()), queue.WAIT)

    def test_a_train_waits_on_it(self):
        self.assertEqual(tip(jobs()), queue.WAIT)

    def test_settled_it_passes_everywhere(self):
        # The control: the same record, every job done, is the pass it became.
        self.assertTrue(judged(SETTLED)[0])
        self.assertEqual(polled(SETTLED), queue.GO)
        self.assertEqual(tip(SETTLED), queue.GO)


class TheSummaryJob(unittest.TestCase):
    def test_only_ci_ok_queued_is_still_unfinished(self):
        evidence = jobs(**{"fmt + clippy + test": DONE})
        self.assertFalse(judged(evidence)[0])
        self.assertEqual(polled(evidence), queue.WAIT)
        self.assertEqual(tip(evidence), queue.WAIT)

    def test_a_finished_run_without_ci_ok_does_not_pass(self):
        evidence = {RUN["id"]: [job for job in SETTLED[RUN["id"]] if job["name"] != "CI ok"]}
        ok, lines = judged(evidence)
        self.assertFalse(ok)
        self.assertIn("`CI ok`", lines[0])
        self.assertEqual(polled(evidence), queue.STOP)
        self.assertEqual(tip(evidence), rails.STALLED)

    def test_ci_ok_that_did_not_succeed_does_not_pass(self):
        for conclusion in ("failure", "skipped", "cancelled"):
            with self.subTest(conclusion=conclusion):
                evidence = jobs(**{"fmt + clippy + test": DONE, "CI ok": {"status": "completed", "conclusion": conclusion}})
                ok, lines = judged(evidence)
                self.assertFalse(ok)
                self.assertIn("`CI ok`", lines[0])
                self.assertNotEqual(polled(evidence), queue.GO)
                self.assertNotEqual(tip(evidence), queue.GO)


if __name__ == "__main__":
    unittest.main()
