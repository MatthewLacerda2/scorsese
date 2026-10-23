#!/usr/bin/env python3
"""`deploy/update` refuses in a sentence, before it changes anything.

The checkout's refusals run against a real git repository.
"""

from __future__ import annotations

import subprocess
import unittest

from .scripted import ENV, GIT, Result, Scripted, scratch, update


class Checkout(unittest.TestCase):
    def repo(self, branch="main", dirty=False):
        host = Scripted([], real_git=True)
        deploy = scratch(env=None)
        root = deploy.parent
        git = ["git", "-C", str(root), "-c", "user.email=t@t", "-c", "user.name=t",
               "-c", "commit.gpgsign=false"]
        subprocess.run(git + ["init", "-q", "-b", branch], check=True)
        (deploy / ".gitignore").write_text(".env\n")
        subprocess.run(git + ["add", "."], check=True)
        subprocess.run(git + ["commit", "-qm", "x"], check=True)
        (deploy / ".env").write_text(ENV)
        if dirty:
            (root / "stray").write_text("x")
        code, _, err = update(host, "--dry-run", deploy=deploy)
        return code, err, host

    def test_another_branch_is_refused_before_anything_runs(self):
        code, err, host = self.repo(branch="feature")
        self.assertEqual(code, 1)
        self.assertIn("on `feature`, not `main`", err)
        self.assertFalse([c for c in host.ran if not c.startswith("git rev-parse")])

    def test_a_dirty_checkout_is_refused_naming_the_files(self):
        code, err, host = self.repo(dirty=True)
        self.assertEqual(code, 1)
        self.assertIn("?? stray", err)
        self.assertFalse([c for c in host.ran if c.startswith("docker")])

    def test_no_settings_file_is_refused(self):
        code, _, err = update(Scripted([]), env=None)
        self.assertEqual(code, 1)
        self.assertIn("no settings file", err)


class Running(unittest.TestCase):
    def test_running_work_refuses_without_wait_before_the_dump(self):
        busy = (["chat_turns"], Result(0, "job\trender\twaiting\t1\n"))
        host = Scripted(GIT + [(["pull"], Result(0)), busy])
        code, _, err = update(host)
        self.assertEqual(code, 1)
        self.assertIn("1 render job waiting", err)
        self.assertIn("--wait", err)
        self.assertFalse([c for c in host.ran if "backup" in c or " up " in c])

    def test_an_unreachable_database_refuses(self):
        down = (["psql"], Result(1, "", "service postgres is not running"))
        code, _, err = update(Scripted(GIT + [(["pull"], Result(0)), down]))
        self.assertEqual(code, 1)
        self.assertIn("is the stack up?", err)


if __name__ == "__main__":
    unittest.main()
