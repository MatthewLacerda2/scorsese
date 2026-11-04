"""`proved_tree.main` writes the answer `changes` reads, and fails toward running (#1758)."""

from __future__ import annotations

import io
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[2]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import proved_tree  # noqa: E402

REPO, SHA, BEFORE = "o/r", "s" * 40, "b" * 40


class Main(unittest.TestCase):
    def run_main(self, env: dict, url_why=None) -> tuple[str, str]:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out"
            out.write_text("", encoding="utf-8")
            argv = ["proved_tree.py", "--sha", SHA, "--before", BEFORE]
            with mock.patch.object(sys, "argv", argv), \
                 mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(out), **env}, clear=True), \
                 mock.patch.object(proved_tree, "verdict", return_value=url_why), \
                 mock.patch("sys.stdout", new_callable=io.StringIO) as printed:
                self.assertEqual(proved_tree.main(), 0)
            return out.read_text(encoding="utf-8"), printed.getvalue()

    def test_a_proved_push_writes_true(self):
        out, printed = self.run_main({"GITHUB_REPOSITORY": REPO}, ("http://run/7", "skips the build: x"))
        self.assertEqual(out, "proved=true\n")
        self.assertIn("skips the build", printed)

    def test_an_unproved_push_writes_false(self):
        out, _ = self.run_main({"GITHUB_REPOSITORY": REPO}, (None, "runs in full: x"))
        self.assertEqual(out, "proved=false\n")

    def test_no_repository_runs_in_full(self):
        out, printed = self.run_main({})
        self.assertEqual(out, "proved=false\n")
        self.assertIn("GITHUB_REPOSITORY", printed)


if __name__ == "__main__":
    unittest.main()
