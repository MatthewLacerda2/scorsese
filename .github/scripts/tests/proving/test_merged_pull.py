"""A merged pull request's own run proves the squash's tree (#1769).

GitHub empties a run's `pull_requests` once its pull request merges, and the
push to `main` comes from that merge, so the run is found through the pushed
commit instead: `commits/{sha}/pulls` names the pull request merged as it, and a
`pull_request` run of that pull request's head counts under the same rules.
"""

from __future__ import annotations

import unittest

from proving.fake import HEAD, REPO, SHA, TREE, Fake, built_jobs, job, pull, verdict


def merged(**kw) -> Fake:
    """A push whose only proving run is its pull request's, as GitHub sends it
    after the merge: `pull_requests` empty."""
    fake = Fake(**kw)
    fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"]["workflow_runs"][0]["pull_requests"] = []
    return fake


class Proves(unittest.TestCase):
    def test_the_merged_pull_requests_run(self):
        url, why = verdict(merged())
        self.assertEqual(url, "http://run/7")
        self.assertIn(f"tree {TREE} was proved by http://run/7", why)

    def test_asks_for_the_pushed_commits_pull_requests(self):
        fake = merged()
        verdict(fake)
        self.assertIn(f"repos/{REPO}/commits/{SHA}/pulls", fake.asked)


class ProvesNothing(unittest.TestCase):
    def assert_runs(self, fake: Fake):
        url, why = verdict(fake)
        self.assertIsNone(url)
        self.assertTrue(why.startswith("runs in full"), why)

    def test_a_pull_request_into_another_branch(self):
        """A stacked pull request merged into `main` through its parent: the
        parent's head is not the stacked one's, whose run tested another base."""
        self.assert_runs(merged(pulls=[pull(base="feature-x")]))

    def test_a_run_of_another_head(self):
        self.assert_runs(merged(pulls=[pull(head="x" * 40)]))

    def test_a_pull_request_not_merged(self):
        self.assert_runs(merged(pulls=[pull(merged=False)]))

    def test_a_pull_request_merged_as_another_commit(self):
        """`commits/{sha}/pulls` also lists a pull request merely containing it."""
        self.assert_runs(merged(pulls=[pull(merge_commit="m" * 40)]))

    def test_no_pull_request(self):
        self.assert_runs(merged(pulls=[]))

    def test_a_listed_base_other_than_main_wins(self):
        """While the run still names its pull request, that base decides."""
        fake = Fake(base="feature-x")
        self.assert_runs(fake)

    def test_a_run_that_did_not_build(self):
        self.assert_runs(merged(jobs=built_jobs(**{"desktop app": job("desktop app", None, "skipped")})))

    def test_the_pulls_lookup_failing_or_malformed(self):
        self.assert_runs(merged(fail=(f"repos/{REPO}/commits/{SHA}/pulls",)))
        self.assert_runs(merged(pulls={"message": "Not Found"}))
        self.assert_runs(merged(pulls=[{"base": None, "head": {}}]))

    def test_a_failed_pulls_lookup_leaves_other_proofs_standing(self):
        """A run on `main` itself still proves the tree."""
        fake = Fake(fail=(f"repos/{REPO}/commits/{SHA}/pulls",))
        fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"]["workflow_runs"][0].update(
            event="push", head_branch="main", pull_requests=[], head_sha=SHA)
        self.assertEqual(verdict(fake)[0], "http://run/7")


if __name__ == "__main__":
    unittest.main()
