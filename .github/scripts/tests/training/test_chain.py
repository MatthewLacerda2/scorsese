"""#1757: the train's chain, on a real repository, is the tree squash-merging its members in order leaves."""

from __future__ import annotations

import functools
import subprocess
import tempfile
import unittest
from pathlib import Path

from .fakes import MIGRATIONS, queue, trains

LINES = [f"line {n}\n" for n in range(20)]


def git(root: str, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True, text=True).stdout.strip()


class Chain(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = tmp.name
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "t")
        git(self.root, "config", "user.email", "t@t")
        (Path(self.root) / MIGRATIONS).mkdir(parents=True)
        self.write("a.txt", "".join(LINES), f"{MIGRATIONS}/0001_first.sql")
        git(self.root, "commit", "-qm", "main")
        git(self.root, "update-ref", "refs/remotes/origin/main", "main")
        self.heads = {}

    def write(self, name: str, text: str, *more: str) -> None:
        (Path(self.root) / name).write_text(text)
        for path in more:
            (Path(self.root) / path).write_text("-- sql\n")
        git(self.root, "add", "-A")

    def branch(self, name: str, at: int, text: str, *more: str) -> str:
        git(self.root, "checkout", "-q", "-b", name, "main")
        lines = list(LINES)
        lines[at] = text
        self.write("a.txt", "".join(lines), *more)
        git(self.root, "commit", "-qm", name)
        git(self.root, "checkout", "-q", "main")
        self.heads[name] = git(self.root, "rev-parse", name)
        return self.heads[name]

    def couple(self, train, number, name):
        run = functools.partial(queue.watch._git, self.root)
        car, why = trains.couple(train, number, self.heads[name], run)
        if car:
            train.cars.append(car)
        return car, why

    def test_the_tip_is_the_tree_squash_merges_in_order_leave(self):
        self.branch("one", 1, "one\n")
        self.branch("two", 15, "two\n", f"{MIGRATIONS}/0002_two.sql")
        train = trains.start(functools.partial(queue.watch._git, self.root))
        self.assertIsNotNone(self.couple(train, 1, "one")[0])
        self.assertIsNotNone(self.couple(train, 2, "two")[0])
        for landed, name in enumerate(("one", "two"), start=1):  # GitHub's squash merges, in order
            git(self.root, "merge", "--squash", "-q", name)
            git(self.root, "commit", "-qm", f"squash {name}")
            self.assertEqual(git(self.root, "rev-parse", "HEAD^{tree}"), train.tree_after(landed))
        self.assertEqual(git(self.root, "rev-parse", "HEAD^{tree}"), train.tree)
        self.assertEqual(git(self.root, "rev-list", "--count", f"{train.base}..{train.tip}"), "2")
        self.assertEqual(git(self.root, "rev-list", "--count", "--merges", train.tip), "0")

    def test_a_conflict_or_a_taken_number_stays_off(self):
        self.branch("one", 1, "one\n", f"{MIGRATIONS}/0002_one.sql")
        self.branch("same-line", 1, "other\n")
        self.branch("same-number", 15, "two\n", f"{MIGRATIONS}/0002_two.sql")
        train = trains.start(functools.partial(queue.watch._git, self.root))
        self.couple(train, 1, "one")
        car, why = self.couple(train, 2, "same-line")
        self.assertEqual((car, why), (None, "it does not merge cleanly onto the train so far"))
        car, why = self.couple(train, 3, "same-number")
        self.assertIsNone(car)
        self.assertIn("0002_two.sql", why)
        self.assertEqual(train.numbers, [1])


if __name__ == "__main__":
    unittest.main()
