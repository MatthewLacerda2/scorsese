#!/usr/bin/env python3
"""Dependabot's pull requests go last, and their branch is never pushed to (#721).

Dependabot stops maintaining a branch somebody else has pushed to, so the one
thing the queue must not do to a bot pull request is the thing it does to every
other: rebase and force-push. These tests hold that, and the order that keeps a
self-labelled bot pull request from taking a turn from the batch's own work.

Nothing here talks to `gh` or `git`.
"""

from __future__ import annotations

import importlib.util
import subprocess
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]

_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)
watch = queue.watch

SHA = "d3b07384d113edec49eaa6238ad5ff00000000aa"
REBASED = "3" * 40
BOT = {"login": "app/dependabot", "is_bot": True}
PULL = {
    "isDraft": False,
    "headRefOid": SHA,
    "headRefName": "dependabot/cargo/cargo-compatible-1a2b3c",
    "number": 801,
    "state": "OPEN",
    "author": BOT,
}


def pr(number: int, *labels: str, author=None, created=None) -> dict:
    return {
        "number": number,
        "isDraft": False,
        "headRefOid": f"{number:040x}",
        "baseRefName": "main",
        "createdAt": created or f"2026-10-03T10:{number % 60:02d}:00Z",
        "author": author or {"login": "MatthewLacerda2"},
        "labels": [{"name": name} for name in labels],
        "closingIssuesReferences": [],
    }


class Order(unittest.TestCase):
    def test_either_spelling_of_dependabot_is_recognised(self):
        for login in ("app/dependabot", "dependabot[bot]"):
            self.assertTrue(watch.is_bot({"author": {"login": login}}))
        self.assertFalse(watch.is_bot({"author": {"login": "MatthewLacerda2"}}))
        self.assertFalse(watch.is_bot({}))

    def test_dependabot_goes_after_an_unlabelled_newer_pull_request(self):
        bot = pr(1, "queue", "dependencies", author=BOT, created="2026-09-01T00:00:00Z")
        plain = pr(2, "queue", created="2026-10-03T00:00:00Z")
        self.assertEqual([p["number"] for p in watch.ordered([bot, plain], {})], [2, 1])

    def test_a_type_label_on_a_bot_pull_request_does_not_lift_it(self):
        # A label is anybody's to add; the author is what decides.
        bot = pr(1, "queue", "infrastructure", author=BOT)
        feature = pr(2, "queue", "feature")
        self.assertEqual(watch.pick([bot, feature], {}, {})["number"], 2)

    def test_a_bot_pull_request_alone_is_still_taken(self):
        self.assertEqual(watch.pick([pr(1, "queue", author=BOT)], {}, {})["number"], 1)


class Branch(unittest.TestCase):
    def take(self, bot_head):
        opts = queue.parse(["801", "--poll", "0"])
        merged = subprocess.CompletedProcess([], 0, "", "")
        with mock.patch.object(queue, "look", return_value=PULL), \
            mock.patch.object(queue, "git"), \
            mock.patch.object(queue, "advance") as rebased, \
            mock.patch.object(queue, "bot_head", return_value=bot_head), \
            mock.patch.object(queue, "wait_for", return_value=(queue.GO, ["CI passed"])) as waited, \
            mock.patch.object(queue.subprocess, "run", return_value=merged), \
            mock.patch("builtins.print"):
            return queue.take("o/r", 801, opts), rebased, waited

    def test_dependabot_is_never_rebased_or_pushed_from_here(self):
        (_, state, _), rebased, _ = self.take((SHA, []))
        rebased.assert_not_called()
        self.assertEqual(state, queue.MERGED)

    def test_the_head_waited_on_is_the_one_dependabot_rebased_to(self):
        _, _, waited = self.take((REBASED, []))
        self.assertEqual(waited.call_args.args[2], REBASED)

    def test_no_rebase_from_dependabot_is_a_hand_back(self):
        (_, state, why), _, waited = self.take((None, ["Dependabot did not rebase."]))
        self.assertEqual(state, queue.HANDED_BACK)
        self.assertIn("Dependabot", why)
        waited.assert_not_called()


class Asking(unittest.TestCase):
    def bot_head(self, tips, seen=REBASED):
        opts = queue.parse(["801", "--poll", "0"])
        with mock.patch.object(queue, "on_tip", side_effect=tips), \
            mock.patch.object(queue, "look", return_value={**PULL, "headRefOid": seen}), \
            mock.patch.object(queue.subprocess, "run") as called, \
            mock.patch.object(queue.time, "sleep"), \
            mock.patch("builtins.print"):
            return queue.bot_head(801, SHA, ".", opts), called

    def test_a_branch_already_on_main_is_not_commented_on(self):
        (fresh, notes), called = self.bot_head([True])
        self.assertEqual((fresh, notes), (SHA, []))
        called.assert_not_called()

    def test_a_branch_behind_main_is_asked_to_rebase_and_waited_for(self):
        (fresh, _), called = self.bot_head([False, True])
        self.assertEqual(fresh, REBASED)
        self.assertIn(queue.BOT_REBASE, called.call_args.args[0])

    def test_a_head_that_has_not_moved_is_not_taken_for_the_rebase(self):
        opts = queue.parse(["801", "--poll", "0", "--deadline", "0.0001"])
        with mock.patch.object(queue, "on_tip", return_value=False), \
            mock.patch.object(queue, "look", return_value=PULL), \
            mock.patch.object(queue.subprocess, "run"), \
            mock.patch("builtins.print"):
            fresh, notes = queue.bot_head(801, SHA, ".", opts)
        self.assertIsNone(fresh)
        self.assertIn("did not rebase", notes[0])


if __name__ == "__main__":
    unittest.main()
