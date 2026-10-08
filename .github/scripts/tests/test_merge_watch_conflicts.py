#!/usr/bin/env python3
"""The watch takes the pull request that conflicts with the fewest others first (#738).

The ordering is pure and tested with the conflicting pairs passed in. The one
function that asks git, `clashing`, is tested against a throwaway repository
built in a temporary directory — local commits only, no remote, no network —
so `merge-tree`'s real exit statuses are what decide.
"""

from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
watch = queue.watch


def pr(number: int, *labels: str, head=None, bot=False):
    return {
        "number": number,
        "isDraft": False,
        "headRefOid": head or f"{number:040x}",
        "baseRefName": "main",
        "createdAt": f"2026-10-03T10:{number % 60:02d}:00Z",
        "author": {"login": "app/dependabot" if bot else "someone"},
        "labels": [{"name": name} for name in labels],
        "closingIssuesReferences": [],
    }


def pair(a: int, b: int) -> frozenset[int]:
    return frozenset((a, b))


class Ordering(unittest.TestCase):
    def test_the_broad_pull_request_goes_after_the_appenders_it_conflicts_with(self):
        # 2026-10-03: #677 conflicted with each of the three; they did not
        # conflict with one another. Landing them first costs one hand-back.
        pulls = [pr(677, "infrastructure"), pr(676, "feature"), pr(686, "feature"), pr(673, "feature")]
        clashes = {pair(677, 676), pair(677, 686), pair(677, 673)}
        order = [p["number"] for p in watch.ordered(pulls, {}, clashes)]
        self.assertEqual(order[-1], 677)

    def test_equal_conflicts_fall_back_to_label_priority_then_age(self):
        pulls = [pr(1, "feature"), pr(2, "infrastructure"), pr(3, "infrastructure")]
        clashes = {pair(1, 2), pair(2, 3), pair(3, 1)}
        self.assertEqual([p["number"] for p in watch.ordered(pulls, {}, clashes)], [2, 3, 1])

    def test_dependabot_stays_last_however_few_its_conflicts(self):
        pulls = [pr(1, "feature"), pr(2, "feature"), pr(9, bot=True)]
        clashes = {pair(1, 2)}
        self.assertEqual(watch.pick(pulls, {}, {}, clashes)["number"], 1)
        self.assertEqual(watch.ordered(pulls, {}, clashes)[-1]["number"], 9)

    def test_a_hand_back_still_on_its_head_counts_against_nobody(self):
        pulls = [pr(1, "infrastructure"), pr(2, "feature"), pr(3, "feature")]
        clashes = {pair(1, 3)}
        # With #3 in line, #1 conflicts and #2 does not: #2 goes first.
        self.assertEqual(watch.pick(pulls, {}, {}, clashes)["number"], 2)
        # #3 was handed back and has not moved: #1 conflicts with nobody in line.
        dropped = {3: f"{3:040x}"}
        self.assertEqual(watch.pick(pulls, dropped, {}, clashes)["number"], 1)

    def test_no_conflicts_is_todays_order(self):
        pulls = [pr(1, "feature"), pr(2, "bug")]
        self.assertEqual(watch.pick(pulls, {}, {}, set())["number"], 2)

    def test_dependabot_is_not_a_contender(self):
        pulls = [pr(1), pr(2, bot=True), pr(3, head="a" * 40)]
        numbers = [p["number"] for p in watch.contenders(pulls, {3: "a" * 40})]
        self.assertEqual(numbers, [1])


class Saying(unittest.TestCase):
    def test_a_reorder_is_said_in_one_line_naming_both_counts(self):
        pulls = [pr(1, "infrastructure"), pr(2, "feature"), pr(3, "feature")]
        line = watch.reordered(pulls, {}, {}, {pair(1, 2), pair(1, 3)})
        self.assertEqual(
            line,
            "#2 goes before #1, which labels alone would take: #2 conflicts with"
            " 1 other(s) in line, #1 with 2.",
        )

    def test_nothing_is_said_when_labels_already_agree(self):
        pulls = [pr(1, "infrastructure"), pr(2, "feature")]
        self.assertIsNone(watch.reordered(pulls, {}, {}, {pair(1, 2)}))
        self.assertIsNone(watch.reordered([], {}, {}, set()))


def git(root, *args):
    # No automatic maintenance: newer git (2.55 on CI's runner, #929) packs a
    # fresh repo in a detached child after `commit`, which can still be
    # writing into `.git` when the test's TemporaryDirectory is removed.
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t",
         "-c", "gc.auto=0", "-c", "maintenance.auto=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()


def commit(root, base, name, text):
    """A commit on `base` writing `text` into `name`; returns its sha."""
    git(root, "checkout", "--quiet", "--detach", base)
    Path(root, name).write_text(text)
    git(root, "add", name)
    git(root, "commit", "--quiet", "-m", name)
    return git(root, "rev-parse", "HEAD")


class Asking(unittest.TestCase):
    def test_merge_tree_decides_and_a_bad_head_fails_open(self):
        with tempfile.TemporaryDirectory() as root:
            git(root, "init", "--quiet")
            Path(root, "shared").write_text("one\n")
            git(root, "add", "shared")
            git(root, "commit", "--quiet", "-m", "base")
            base = git(root, "rev-parse", "HEAD")
            a = commit(root, base, "shared", "two\n")
            b = commit(root, base, "shared", "three\n")
            c = commit(root, base, "other", "four\n")
            pulls = [pr(1, head=a), pr(2, head=b), pr(3, head=c), pr(4, head="f" * 40)]
            known = {}
            self.assertEqual(watch.clashing(pulls, root, known), {pair(1, 2)})
            # Asked once per pair of heads: a second poll runs no git at all.
            with mock.patch.object(watch.subprocess, "run") as ran:
                self.assertEqual(watch.clashing(pulls, root, known), {pair(1, 2)})
            ran.assert_not_called()

    def test_exit_one_without_a_tree_is_an_error_not_a_conflict(self):
        # git 2.43 exits 1 for "not something we can merge" too.
        self.assertTrue(watch.conflicted(1, "a" * 40 + "\nshared\n"))
        self.assertFalse(watch.conflicted(1, ""))
        self.assertFalse(watch.conflicted(0, "a" * 40 + "\n"))
        self.assertFalse(watch.conflicted(129, "usage: git merge-tree"))

    def test_git_that_cannot_run_counts_as_no_conflict(self):
        pulls = [pr(1), pr(2)]
        with mock.patch.object(watch.subprocess, "run", side_effect=OSError("no git")):
            self.assertEqual(watch.clashing(pulls, "/nonexistent", {}), set())


class Watching(unittest.TestCase):
    def test_the_watch_takes_the_appenders_first_and_says_why(self):
        broad, small, other = pr(1, "infrastructure"), pr(2, "feature"), pr(3, "feature")
        listings = iter([[broad, small, other], [broad, other], [other], []])
        said, clock = [], [0.0]
        opts = queue.parse(["--watch", "--for", "30"])

        def take(repo, number, opts, heads=None, tick=None):
            clock[0] += 120
            return number, queue.MERGED, "why"

        def gh(*args):
            # Only the `queue` listing walks the script; the board (#819) is empty.
            return next(listings, []) if "--label" in args else []

        with mock.patch.object(queue.mergeable, "gh", side_effect=gh), \
            mock.patch("builtins.print"), \
            mock.patch.object(watch, "clashing", return_value={pair(1, 2), pair(1, 3)}), \
            mock.patch.object(queue, "take", side_effect=take) as took, \
            mock.patch.object(queue.time, "monotonic", side_effect=lambda: clock[0]), \
            mock.patch.object(queue.time, "sleep", side_effect=lambda s: clock.__setitem__(0, clock[0] + s)), \
            mock.patch.object(queue, "say", side_effect=lambda line, *r: said.append(line)):
            queue.run_watch("o/r", opts)
        # #2 conflicts with nobody and goes first; #1 and #3 then conflict only
        # with each other, a tie that the label breaks.
        self.assertEqual([call.args[1] for call in took.call_args_list], [2, 1, 3])
        self.assertIn("#2 goes before #1", " ".join(said))

if __name__ == "__main__":
    unittest.main()
