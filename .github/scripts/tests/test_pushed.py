#!/usr/bin/env python3
"""`make pushed` agrees only when the last local commit is the branch's head on origin.

A coder's report once said pushed while its last commit never reached the pull
request. `tools/pushed` is the check run after every push; these drive the real
target in a scratch clone of a bare repository standing in for GitHub.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# A commit needs an author; the host's git config decides nothing here.
GIT_ENV = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@example.com",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@example.com",
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
}


@unittest.skipUnless(shutil.which("make") and shutil.which("git"), "make or git missing")
class Pushed(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.dir)
        self.env = {**os.environ, **GIT_ENV}
        self.git("init", "--quiet", "--bare", str(self.dir / "origin.git"))
        self.clone = self.dir / "clone"
        self.git("clone", "--quiet", str(self.dir / "origin.git"), str(self.clone))
        self.git("switch", "--quiet", "-c", "infra-example", cwd=self.clone)
        self.commit("first")

    def git(self, *args: str, cwd: Path | None = None) -> None:
        subprocess.run(["git", *args], cwd=cwd, env=self.env, check=True, capture_output=True)

    def commit(self, message: str) -> None:
        self.git("commit", "--quiet", "--allow-empty", "-m", message, cwd=self.clone)

    def pushed(self) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["make", "-s", "--no-print-directory", "-C", str(ROOT), "pushed", f"REPO={self.clone}"],
            env=self.env,
            capture_output=True,
            text=True,
        )

    def test_a_branch_never_pushed_is_refused(self) -> None:
        run = self.pushed()
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertIn("origin has no branch infra-example", run.stderr)
        self.assertIn("git push -u origin infra-example", run.stderr)

    def test_a_commit_after_the_push_is_refused(self) -> None:
        self.git("push", "--quiet", "-u", "origin", "infra-example", cwd=self.clone)
        self.commit("second, never pushed")
        run = self.pushed()
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertIn("the last commit has not reached the pull request", run.stderr)

    def test_the_pushed_head_agrees(self) -> None:
        self.commit("second")
        self.git("push", "--quiet", "-u", "origin", "infra-example", cwd=self.clone)
        run = self.pushed()
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("is the head on origin", run.stdout)


if __name__ == "__main__":
    unittest.main()
