"""`proved_tree.verdict` skips only on a tree a run really built (#1758).

Every way the lookup can be wrong must run the gates in full.
"""

from __future__ import annotations

import unittest

from proving.fake import REPO, SHA, BEFORE, TREE, Fake, built_jobs, job, proved_tree, verdict  # noqa: F401


class Skips(unittest.TestCase):
    def test_an_identical_tree_with_a_run_that_built(self):
        url, why = verdict(Fake())
        self.assertEqual(url, "http://run/7")
        self.assertIn("skips the build", why)
        self.assertIn(TREE, why)

    def test_a_proving_run_on_the_second_page(self):
        fake = Fake()
        other = {"id": 1, "html_url": "http://run/1", "head_commit": {"tree_id": "x" * 40}}
        first = {"workflow_runs": [other] * proved_tree.PER_PAGE}
        found = {"workflow_runs": [{"id": 7, "html_url": "http://run/7", "head_commit": {"tree_id": TREE},
                                    "event": "push", "head_branch": "main"}]}
        real = fake.__call__

        def paged(path: str):
            if path.startswith(f"repos/{REPO}/actions/workflows/"):
                return found if path.endswith("&page=2") else first
            return real(path)

        self.assertEqual(proved_tree.verdict(paged, REPO, SHA, BEFORE)[0], "http://run/7")


class RunsInFull(unittest.TestCase):
    def assert_runs(self, fake: Fake, **kw):
        url, why = verdict(fake, **kw)
        self.assertIsNone(url)
        self.assertTrue(why.startswith("runs in full"), why)

    def test_a_different_tree(self):
        """A hand push, or a merge from GitHub's page of a head no run built."""
        self.assert_runs(Fake(tree="x" * 40))

    def test_a_markdown_only_run_whose_heavy_jobs_skipped(self):
        skipped = {name: job(name, None, "skipped") for name in
                   ("fmt + clippy + test", "desktop app", "web front-end", "deploy config", "cargo-deny")}
        self.assert_runs(Fake(jobs=built_jobs(**skipped)))

    def test_a_run_against_another_base(self):
        """A stacked pull request's run tested its head merged into that base."""
        self.assert_runs(Fake(base="feature-x"))

    def test_a_push_run_off_main(self):
        fake = Fake()
        fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"]["workflow_runs"][0].update(
            event="push", head_branch="feature-x", pull_requests=[])
        self.assert_runs(fake)

    def test_a_draft_run_whose_jobs_skipped(self):
        self.assert_runs(Fake(jobs=[job("changes", None, "skipped"), job("CI ok", None, "skipped")]))

    def test_a_job_missing_from_the_run(self):
        self.assert_runs(Fake(jobs=built_jobs(**{"cargo-deny": None})))

    def test_a_job_whose_checking_step_skipped(self):
        self.assert_runs(Fake(jobs=built_jobs(**{"fmt + clippy + test": job(
            "fmt + clippy + test", "Test", step_concl="skipped")})))

    def test_a_failed_job(self):
        self.assert_runs(Fake(jobs=built_jobs(**{"desktop app": job("desktop app", "Test", "failure", "failure")})))

    def test_a_change_to_a_cache_key_or_the_workflow(self):
        for path in ("Cargo.lock", "app/Cargo.lock", "tools/lint/Cargo.toml", "rust-toolchain.toml",
                     ".cargo/config.toml", "app/.cargo/config.toml", ".cargo/config", "tools/ci-apt", "tools/ci-postgres", "tools/chromium/pin",
                     ".github/workflows/ci.yml"):
            with self.subTest(path=path):
                url, why = verdict(Fake(files=("crates/core/src/lib.rs", path)))
                self.assertIsNone(url)
                self.assertIn(path, why)

    def test_a_key_renamed_away(self):
        fake = Fake()
        fake.answers[f"repos/{REPO}/compare/"]["files"] = [
            {"filename": ".cargo/cargo-config.toml", "previous_filename": ".cargo/config.toml"}]
        url, why = verdict(fake)
        self.assertIsNone(url)
        self.assertIn(".cargo/config.toml", why)

    def test_a_path_only_named_like_a_key_is_not_one(self):
        for path in ("docs/Cargo.lock.md", "tools/ci-apt-notes", ".github/workflows/coverage.yml",
                     ".cargo/mutants.toml", "docs/.cargo/config.toml.md"):
            with self.subTest(path=path):
                self.assertFalse(proved_tree.keyed(path))

    def test_a_push_that_is_not_a_fast_forward_or_has_no_before(self):
        self.assert_runs(Fake(status="diverged"))
        self.assert_runs(Fake(), before=proved_tree.NO_COMMIT)
        self.assert_runs(Fake(), before="")
        self.assert_runs(Fake(files=[f"f{i}" for i in range(proved_tree.COMPARE_CAP)]))

    def test_any_api_error(self):
        for prefix in (f"repos/{REPO}/compare/", f"repos/{REPO}/commits/",
                       f"repos/{REPO}/actions/workflows/", f"repos/{REPO}/actions/runs/"):
            with self.subTest(prefix=prefix):
                self.assert_runs(Fake(fail=(prefix,)))

    def test_a_malformed_answer(self):
        fake = Fake()
        fake.answers[f"repos/{REPO}/commits/{SHA}"] = {"commit": {}}
        self.assert_runs(fake)
        fake = Fake()
        fake.answers[f"repos/{REPO}/actions/workflows/ci.yml/"] = {"workflow_runs": [{"head_commit": None}]}
        self.assert_runs(fake)


if __name__ == "__main__":
    unittest.main()
