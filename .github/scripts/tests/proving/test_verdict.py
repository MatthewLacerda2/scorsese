"""`proved_tree.verdict` skips only on a tree a run really built (#1758).

A fake `gh api` answers from canned bodies keyed by path prefix, so nothing
reaches GitHub. Every way the lookup can be wrong must run the gates in full.
"""

from __future__ import annotations

import io
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import proved_tree  # noqa: E402

REPO, SHA, BEFORE, TREE = "o/r", "s" * 40, "b" * 40, "t" * 40


def job(name: str, step: str | None, conclusion: str = "success", step_concl: str = "success") -> dict:
    steps = [{"name": "Set up job", "conclusion": "success"}]
    if step:
        steps.append({"name": step, "conclusion": step_concl})
    return {"name": name, "conclusion": conclusion, "steps": steps}


def built_jobs(**over) -> list[dict]:
    """Every job `BUILT` names, each with its step run; `over` replaces by name."""
    jobs = {name: job(name, step) for name, step in proved_tree.BUILT.items()}
    jobs.update(over)
    return [j for j in jobs.values() if j is not None]


class Fake:
    """`gh api`: the answer for the first prefix a path starts with, else `None`."""

    def __init__(self, jobs=None, tree=TREE, files=("crates/core/src/lib.rs",), status="ahead", fail=(), tip=SHA):
        run = {"id": 7, "html_url": "http://run/7", "head_commit": {"tree_id": tree}}
        self.answers = {
            f"repos/{REPO}/compare/": {"status": status, "files": [{"filename": f} for f in files]},
            f"repos/{REPO}/commits/main": {"sha": tip},
            f"repos/{REPO}/commits/{SHA}": {"commit": {"tree": {"sha": TREE}}},
            f"repos/{REPO}/actions/workflows/ci.yml/": {"workflow_runs": [run]},
            f"repos/{REPO}/actions/runs/7/jobs": {"jobs": built_jobs() if jobs is None else jobs},
        }
        self.fail = fail
        self.asked: list[str] = []

    def __call__(self, path: str):
        self.asked.append(path)
        for prefix, answer in self.answers.items():
            if path.startswith(prefix):
                return None if any(prefix.startswith(f) for f in self.fail) else answer
        return None


def verdict(fake: Fake, before: str = BEFORE):
    return proved_tree.verdict(fake, REPO, SHA, before)


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
        found = {"workflow_runs": [{"id": 7, "html_url": "http://run/7", "head_commit": {"tree_id": TREE}}]}
        real = fake.__call__

        def paged(path: str):
            if path.startswith(f"repos/{REPO}/actions/workflows/"):
                return found if path.endswith("&page=2") else first
            return real(path)

        self.assertEqual(proved_tree.verdict(paged, REPO, SHA, BEFORE)[0], "http://run/7")


class Superseded(unittest.TestCase):
    """A train's intermediate squash is no longer `main`'s tip by its push run."""

    def test_a_push_main_already_left_skips_without_a_run(self):
        fake = Fake(tree="x" * 40, tip="n" * 40)
        url, why = verdict(fake)
        self.assertEqual(url, "n" * 40)
        self.assertIn("skips the build", why)
        self.assertFalse(any("/actions/" in p for p in fake.asked), "no run is needed to skip it")

    def test_a_push_still_the_tip_needs_its_own_proof(self):
        self.assertIsNone(verdict(Fake(tree="x" * 40))[0])

    def test_a_tip_lookup_failing_is_not_superseded(self):
        for fake in (Fake(tree="x" * 40, fail=(f"repos/{REPO}/commits/main",)), Fake(tree="x" * 40, tip="short")):
            with self.subTest():
                url, why = verdict(fake)
                self.assertIsNone(url)
                self.assertTrue(why.startswith("runs in full"), why)

    def test_a_superseded_push_that_moves_a_cache_key_still_runs(self):
        self.assertIsNone(verdict(Fake(files=("Cargo.lock",), tip="n" * 40))[0])


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
                     ".cargo/config.toml", "tools/ci-apt", "tools/ci-postgres", "tools/chromium/pin",
                     ".github/workflows/ci.yml"):
            with self.subTest(path=path):
                url, why = verdict(Fake(files=("crates/core/src/lib.rs", path)))
                self.assertIsNone(url)
                self.assertIn(path, why)

    def test_a_path_only_named_like_a_key_is_not_one(self):
        for path in ("docs/Cargo.lock.md", "tools/ci-apt-notes", ".github/workflows/coverage.yml"):
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


class Main(unittest.TestCase):
    def run_main(self, env: dict, url_why=None) -> tuple[str, str]:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out"
            out.write_text("", encoding="utf-8")
            argv = ["proved_tree.py", "--sha", SHA, "--before", BEFORE]
            with mock.patch.object(sys, "argv", argv), \
                 mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(out), **env}, clear=True), \
                 mock.patch.object(proved_tree, "verdict", return_value=url_why), \
                 mock.patch("sys.stdout", new_callable=io.StringIO) as printed:
                self.assertEqual(proved_tree.main(), 0)
            return out.read_text(encoding="utf-8"), printed.getvalue()

    def test_a_proved_push_writes_true(self):
        out, printed = self.run_main({"GITHUB_REPOSITORY": REPO}, ("http://run/7", "skips the build: x"))
        self.assertEqual(out, "proved=true\n")
        self.assertIn("skips the build", printed)

    def test_an_unproved_push_writes_false(self):
        out, _ = self.run_main({"GITHUB_REPOSITORY": REPO}, (None, "runs in full: x"))
        self.assertEqual(out, "proved=false\n")

    def test_no_repository_runs_in_full(self):
        out, printed = self.run_main({})
        self.assertEqual(out, "proved=false\n")
        self.assertIn("GITHUB_REPOSITORY", printed)


if __name__ == "__main__":
    unittest.main()
