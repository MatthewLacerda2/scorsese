#!/usr/bin/env python3
"""A pull request that conflicts with `main` is handed back at once (#1345).

Its turn could only end in that hand-back, so the watch has it now, files
named, instead of keeping it in line. `against_main` is tested against a
throwaway repository whose `origin/main` is a local ref — no remote, no
network — so `merge-tree`'s real output is what decides.
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


def pr(number: int, head=None, bot=False, mergeable="MERGEABLE"):
    return {
        "number": number,
        "isDraft": False,
        "headRefOid": head or f"{number:040x}",
        "baseRefName": "main",
        "createdAt": f"2026-10-15T10:{number % 60:02d}:00Z",
        "author": {"login": "app/dependabot" if bot else "someone"},
        "labels": [],
        "closingIssuesReferences": [],
        "mergeable": mergeable,
    }


def git(root, *args):
    # No automatic maintenance, as in `test_conflicts.py` (#929).
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t",
         "-c", "gc.auto=0", "-c", "maintenance.auto=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()


def commit(root, base, files):
    git(root, "checkout", "--quiet", "--detach", base)
    for name, text in files.items():
        Path(root, name).write_text(text)
        git(root, "add", name)
    git(root, "commit", "--quiet", "-m", "c")
    return git(root, "rev-parse", "HEAD")


class Asking(unittest.TestCase):
    def test_merge_tree_names_the_conflicting_paths_against_origin_main(self):
        with tempfile.TemporaryDirectory() as root:
            git(root, "init", "--quiet")
            Path(root, "a").write_text("1\n")
            Path(root, "b").write_text("1\n")
            git(root, "add", "a", "b")
            git(root, "commit", "--quiet", "-m", "base")
            base = git(root, "rev-parse", "HEAD")
            main = commit(root, base, {"a": "main\n", "b": "main\n"})
            git(root, "update-ref", "refs/remotes/origin/main", main)
            stale = commit(root, base, {"a": "pr\n", "b": "pr\n"})
            fine = commit(root, base, {"c": "new\n"})
            pulls = [pr(1, head=stale), pr(2, head=fine), pr(3, head="f" * 40)]
            known = {}
            self.assertEqual(watch.against_main(pulls, root, known), {1: ["a", "b"]})
            self.assertEqual(len(known), 3)  # the bad head fails open, once

    def test_no_origin_main_is_no_conflict(self):
        with tempfile.TemporaryDirectory() as root:
            git(root, "init", "--quiet")
            self.assertEqual(watch.against_main([pr(1)], root, {}), {})

    def test_paths_are_read_up_to_gits_messages(self):
        out = "a" * 40 + "\nb\na\na\n\nAuto-merging a\nCONFLICT (content)\n"
        self.assertEqual(watch.conflict_paths(1, out), ["a", "b"])
        self.assertIsNone(watch.conflict_paths(0, "a" * 40 + "\n"))


class Deciding(unittest.TestCase):
    def test_either_git_or_github_is_enough_and_dependabot_is_left_out(self):
        pulls = [pr(1), pr(2, mergeable="CONFLICTING"), pr(3), pr(4, bot=True, mergeable="CONFLICTING")]
        said = watch.behind(pulls, {}, {1: ["x.rs"], 4: ["y"]})
        self.assertEqual(said, [
            (1, "it conflicts with `main` in `x.rs`; rebase it and push."),
            (2, "it conflicts with `main` (GitHub says so); rebase it and push."),
        ])

    def test_the_watch_hands_it_back_without_taking_it_and_takes_the_rest(self):
        clock, results = [0.0], None
        listing = [pr(1), pr(2)]

        def take(repo, number, opts, heads=None, tick=None):
            clock[0] += 600
            return number, queue.MERGED, "why"

        gh = lambda *args: listing if "--label" in args else []  # noqa: E731
        opts = queue.parse(["--watch", "--for", "20"])
        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch.object(queue, "take", side_effect=take) as took, \
            mock.patch.object(watch, "clashing", return_value=set()), \
            mock.patch.object(watch, "against_main", return_value={1: ["x.rs"]}), \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=lambda s: clock.__setitem__(0, clock[0] + s)), \
            mock.patch("builtins.print"):
            results = queue.run_watch("o/r", opts)
        self.assertEqual([call.args[1] for call in took.call_args_list], [2])
        self.assertEqual(results[0], (1, queue.HANDED_BACK,
                                      "it conflicts with `main` in `x.rs`; rebase it and push."))
        # Remembered by head like any hand-back: never handed back twice.
        self.assertEqual(sum(1 for r in results if r[0] == 1), 1)


if __name__ == "__main__":
    unittest.main()
