"""A merge train's dispatched run proves the tree its last squash leaves (#1765).

The queue tests a train by `workflow_dispatch` on a `merge-train/*` branch
(#1757); the last member's push to `main` has exactly that run's tree. Only a
dispatch on the train's own prefix counts, under the same `BUILT` rule.
"""

from __future__ import annotations

import unittest

from proving.fake import REPO, TREE, Fake, built_jobs, job, proved_tree, verdict  # noqa: F401

from merge_queue import train  # noqa: E402  (`proving.fake` puts the scripts on the path)


def dispatched(branch: str, jobs=None) -> Fake:
    fake = Fake(jobs=jobs)
    fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"]["workflow_runs"][0].update(
        event="workflow_dispatch", head_branch=branch, pull_requests=[])
    return fake


class TrainRun(unittest.TestCase):
    def test_a_train_run_proves_its_tree(self):
        url, why = verdict(dispatched(f"{train.PREFIX}abc1234-12-13"))
        self.assertEqual(url, "http://run/7")
        self.assertIn(f"tree {TREE} was proved by http://run/7", why)

    def test_a_dispatch_on_another_branch_does_not(self):
        for branch in ("main", "feature-x", "merge-train", "x/merge-train/abc", None):
            with self.subTest(branch=branch):
                url, why = verdict(dispatched(branch))
                self.assertIsNone(url)
                self.assertTrue(why.startswith("runs in full"), why)

    def test_a_train_run_with_a_job_skipped_does_not(self):
        skipped = built_jobs(**{"desktop app": job("desktop app", None, "skipped")})
        url, why = verdict(dispatched(f"{train.PREFIX}abc1234-12", jobs=skipped))
        self.assertIsNone(url)
        self.assertTrue(why.startswith("runs in full"), why)

    def test_another_event_on_a_train_branch_does_not(self):
        """Only the queue dispatches there; a push run on one proves nothing."""
        fake = dispatched(f"{train.PREFIX}abc1234-12")
        fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"]["workflow_runs"][0]["event"] = "push"
        self.assertIsNone(verdict(fake)[0])

    def test_the_prefix_is_the_queues_own(self):
        self.assertIs(proved_tree.TRAIN_PREFIX, train.PREFIX)


if __name__ == "__main__":
    unittest.main()
