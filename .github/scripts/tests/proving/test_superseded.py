"""`proved_tree.verdict` on a push `main` has already moved past (#1758)."""

from __future__ import annotations

import unittest

from proving.fake import REPO, SHA, BEFORE, TREE, Fake, built_jobs, job, proved_tree, verdict  # noqa: F401


class Superseded(unittest.TestCase):
    """A train's intermediate squash is no longer `main`'s tip by its push run."""

    def test_a_push_main_already_left_skips_without_a_run(self):
        fake = Fake(tree="x" * 40, tip="n" * 40)
        url, why = verdict(fake)
        self.assertEqual(url, "n" * 40)
        self.assertIn("skips the build", why)
        self.assertFalse(any("/actions/" in p for p in fake.asked), "no run is needed to skip it")

    def test_a_push_followed_only_by_markdown_needs_its_own_proof(self):
        """The Markdown-only tip's run asks nothing and builds nothing."""
        url, why = verdict(Fake(tree="x" * 40, tip="n" * 40, later=("docs/notes.md",)))
        self.assertIsNone(url)
        self.assertTrue(why.startswith("runs in full"), why)

    def test_a_push_followed_by_unreadable_changes_needs_its_own_proof(self):
        fake = Fake(tree="x" * 40, tip="n" * 40)
        fake.answers[f"repos/{REPO}/compare/{SHA}..."] = {"status": "diverged", "files": []}
        self.assertIsNone(verdict(fake)[0])

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


if __name__ == "__main__":
    unittest.main()
