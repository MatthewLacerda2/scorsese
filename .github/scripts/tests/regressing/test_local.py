"""`make regression`: the `regression test` job, run here before readying (#1701).

The same throwaway repository as `test_on_base.py` (the base has the bug, the
head fixes it), run through `regression_local.py` the way a coder runs it:
from the checkout, against `BASE` or a pull request's base, in a scratch
worktree that is gone afterwards.
"""

from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path
from unittest import mock

import sys

from .test_on_base import CATCHES, MISSES, NEW_API, SCRIPTS, git, quietly, regression, repo

if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import regression_local  # noqa: E402


def worktrees(root: Path) -> int:
    listed = subprocess.run(["git", "worktree", "list", "--porcelain"], cwd=root,
                            check=True, text=True, stdout=subprocess.PIPE).stdout
    return listed.count("worktree ")


class Local(unittest.TestCase):
    def local(self, root: Path, *argv: str, base: str = "HEAD^1", gh=None) -> int:
        env = {k: v for k, v in os.environ.items()
               if k not in ("GITHUB_EVENT_NAME", "GITHUB_OUTPUT", "BASE")}
        if base:
            env["BASE"] = base
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(Path, "cwd", return_value=root):
            return quietly(regression_local.local, list(argv), gh)

    def test_a_test_that_fails_on_the_base_passes(self) -> None:
        root = repo(self, CATCHES)
        self.assertEqual(self.local(root), 0)
        self.assertEqual(worktrees(root), 1, "the scratch worktree is removed")

    def test_a_test_that_passes_on_the_base_is_refused(self) -> None:
        self.assertEqual(self.local(repo(self, MISSES)), 1)

    def test_a_test_naming_what_the_fix_added_is_refused(self) -> None:
        self.assertEqual(self.local(repo(self, NEW_API)), 1)

    def test_a_dirty_tree_is_refused_before_anything_runs(self) -> None:
        root = repo(self, CATCHES)
        (root / "calc.py").write_text("def double(x):\n    return 0\n")
        with mock.patch.object(regression, "on_base") as ran:
            self.assertEqual(self.local(root), 2)
        ran.assert_not_called()

    def test_no_changed_test_is_refused(self) -> None:
        root = repo(self, CATCHES)
        git(root, "rm", "-q", "tests/test_calc.py")
        git(root, "commit", "-qm", "no test", "--amend")
        self.assertEqual(self.local(root), 1)

    def test_without_a_base_or_a_pull_request_it_asks_for_one(self) -> None:
        self.assertEqual(self.local(repo(self, CATCHES), base=""), 2)


class FromAPullRequest(unittest.TestCase):
    """`PR=N` reads the description as the job's `plan` does."""

    def gh(self, body: str, labels: tuple[str, ...]):
        answers = {"repos/o/r/pulls/9": {"body": body, "base": {"ref": "main"}},
                   "repos/o/r/issues/7": {"state": "open", "labels": [{"name": n} for n in labels]}}
        return lambda _api, path: answers[path]

    def local(self, root: Path, body: str, labels: tuple[str, ...]) -> int:
        with mock.patch.object(regression_local, "repository", return_value="o/r"):
            return Local.local(self, root, "9", gh=self.gh(body, labels))

    def test_a_bug_fix_runs_on_the_base(self) -> None:
        self.assertEqual(self.local(repo(self, MISSES), "Closes #7", ("bug",)), 1)
        self.assertEqual(self.local(repo(self, CATCHES), "Closes #7", ("bug",)), 0)

    def test_a_fix_that_is_not_a_bug_owes_nothing_and_runs_nothing(self) -> None:
        with mock.patch.object(regression, "on_base") as ran:
            self.assertEqual(self.local(repo(self, MISSES), "Closes #7", ("feature",)), 0)
        ran.assert_not_called()

    def test_the_written_exception_owes_nothing(self) -> None:
        body = "Closes #7\nNo regression test on base: only real speakers show the click.\n"
        self.assertEqual(self.local(repo(self, MISSES), body, ("bug",)), 0)

    def test_a_checkout_that_is_not_the_pull_requests_head_is_refused(self) -> None:
        answers = {"repos/o/r/pulls/9": {"body": "Closes #7", "head": {"sha": "0" * 40}}}
        with mock.patch.object(regression_local, "repository", return_value="o/r"), \
                mock.patch.object(regression, "on_base") as ran:
            code = Local.local(self, repo(self, CATCHES), "9", gh=lambda _api, path: answers[path])
        self.assertEqual(code, 2)
        ran.assert_not_called()

    def test_github_out_of_reach_is_cannot_ask_not_a_refusal(self) -> None:
        def down(_api, _path):
            raise regression_local.Unreachable("network: GitHub unreachable")
        with mock.patch.object(regression_local, "repository", return_value="o/r"):
            self.assertEqual(Local.local(self, repo(self, CATCHES), "9", gh=down), 2)


if __name__ == "__main__":
    unittest.main()
