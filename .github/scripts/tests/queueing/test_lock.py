#!/usr/bin/env python3
"""One queue at a time per repository (#1346).

A real `flock` on a file in a temporary directory: a second open of the same
file conflicts even within one process, so both sides run here.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
lock = queue.lock

START = datetime(2026, 10, 15, 3, 0, tzinfo=timezone.utc)


class Locking(unittest.TestCase):
    def test_a_second_acquire_is_refused_naming_the_holder_until_it_lets_go(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root, lock.NAME)
            first = lock.acquire(path, START)
            with self.assertRaises(lock.Held) as refused:
                lock.acquire(path, START + timedelta(minutes=50))
            self.assertIn(f"pid {os.getpid()}, running for 50 minutes", str(refused.exception))
            first.close()
            lock.acquire(path, START).close()

    def test_an_unwritten_or_unreadable_holder_is_still_named(self):
        self.assertIn("not written yet", lock.holder("", START))
        self.assertEqual(lock.holder("42 garbage", START), "pid 42")

    def test_every_worktree_of_a_clone_shares_one_lock(self):
        with tempfile.TemporaryDirectory() as root:
            run = lambda *a, cwd=root: subprocess.run(  # noqa: E731
                ["git", "-c", "user.name=t", "-c", "user.email=t@t", *a],
                cwd=cwd, check=True, capture_output=True)
            run("init", "--quiet")
            run("commit", "--quiet", "--allow-empty", "-m", "base")
            run("worktree", "add", "--quiet", "--detach", str(Path(root, "wt")))
            self.assertEqual(lock.path_for(root), lock.path_for(str(Path(root, "wt"))))
            self.assertEqual(lock.path_for(root), Path(root, ".git", lock.NAME).resolve())

    def test_a_held_lock_stops_the_queue_before_it_asks_github_anything(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root, lock.NAME)
            held = lock.acquire(path)
            with mock.patch.object(lock, "path_for", return_value=path), \
                mock.patch.object(queue.mergeable, "gh") as gh, \
                mock.patch("builtins.print"):
                status = queue.main(["1"])
            held.close()
        self.assertEqual(status, queue.LOCKED_STATUS)
        gh.assert_not_called()


if __name__ == "__main__":
    unittest.main()
