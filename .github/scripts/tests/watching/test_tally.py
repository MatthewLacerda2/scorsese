#!/usr/bin/env python3
"""The aging tally outlives one watch (#1423).

A tally kept in memory started over at every `watch: resume:`. The file's round
trip, its place beside the lock and `main`'s choice of file are tested here;
the count carrying across resumed watches is in `test_aging.py`.
"""

from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
watch = queue.watch
tally = queue.tally


class Carrying(unittest.TestCase):
    """The tally outlives one watch (#1423): written each pick, read at start."""

    def test_a_tally_written_by_one_run_is_read_by_the_next(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root, tally.NAME)
            tally.save(path, {1: 2, 7: 1})
            self.assertEqual(tally.load(path), {1: 2, 7: 1})

    def test_a_missing_or_unreadable_tally_is_an_empty_one(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root, tally.NAME)
            self.assertEqual(tally.load(path), {})
            path.write_text("{not json", encoding="utf-8")
            self.assertEqual(tally.load(path), {})
            path.write_text('{"1": "three"}', encoding="utf-8")
            self.assertEqual(tally.load(path), {})

    def test_a_failed_save_leaves_no_spare_behind(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root, tally.NAME)
            path.mkdir()  # the rename onto a directory fails
            tally.save(path, {1: 2})
            self.assertEqual(sorted(p.name for p in Path(root).iterdir()), [tally.NAME])

    def test_every_worktree_shares_the_file_in_the_common_git_dir(self):
        with tempfile.TemporaryDirectory() as root:
            clone, other = Path(root, "clone"), Path(root, "other")
            git = lambda *args: subprocess.run(  # noqa: E731
                ["git", *args], cwd=clone, check=True, capture_output=True
            )
            clone.mkdir()
            git("init", "-q")
            git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x")
            git("worktree", "add", "-q", str(other))
            path = tally.path_for(str(other))
            self.assertEqual(path, (clone / ".git" / tally.NAME).resolve())
            self.assertEqual(tally.path_for(str(clone)), path)

    def test_another_label_keeps_its_own_file(self):
        with tempfile.TemporaryDirectory() as root:
            subprocess.run(["git", "init", "-q", root], check=True)
            queue_file = tally.path_for(root)
            self.assertEqual(tally.path_for(root, watch.QUEUE_LABEL), queue_file)
            other = tally.path_for(root, "hot fix")
            self.assertEqual(other.parent, queue_file.parent)
            self.assertEqual(other.name, "merge-watch-tally-hot-fix.json")

    def test_main_hands_the_watch_its_labels_file_and_a_dry_run_none(self):
        def started(*argv):
            with mock.patch.object(queue.mergeable, "gh", return_value={"nameWithOwner": "o/r"}), \
                mock.patch.object(queue.lock, "acquire"), \
                mock.patch.object(queue, "run_watch", return_value=[]) as run, \
                mock.patch("builtins.print"):
                queue.main(["--watch", *argv])
            return run.call_args.args

        root = queue.parse(["--watch"]).root
        self.assertEqual(started(), ("o/r", mock.ANY, tally.path_for(root)))
        self.assertEqual(
            started("--label", "other"), ("o/r", mock.ANY, tally.path_for(root, "other"))
        )
        self.assertEqual(started("--no-merge"), ("o/r", mock.ANY, None))


if __name__ == "__main__":
    unittest.main()
