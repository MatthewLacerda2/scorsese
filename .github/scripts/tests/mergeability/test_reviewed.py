"""A pull request that changes money, data, security, concurrency, render or the
gates is handed back without an independent review at its level (#1498).

`reviewed.py` is pure but for `evidence`, and `mergeable.judge` reads its
answer from `pull["reviewed"]`, so all of it is tested by import.
"""

from __future__ import annotations

import fnmatch
import importlib.util
import subprocess
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "mergeable.py"
ROOT = Path(__file__).resolve().parents[4]
_spec = importlib.util.spec_from_file_location("mergeable", SCRIPT)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)
reviewed = mergeable.reviewed

SHA = "c23b7c4f0000000000000000000000000000000a"
GREEN = [{"name": "CI", "head_sha": SHA, "id": 1, "status": "completed",
          "conclusion": "success", "html_url": "https://example.invalid/1"}]
JOBS = {1: [{"name": "fmt + clippy + test", "conclusion": "success"}]}
MONEY = "crates/providers/src/spending/ledger.rs"
JOBS_RS = "crates/server/src/jobs/worker.rs"


def changed(*paths: str) -> list[dict]:
    return [{"filename": path, "status": "modified"} for path in paths]


def pull(files: list[dict], comments: list[str] = (), body: str = "Gates: green",
         labels: list[str] = (), author: str = "MatthewLacerda2") -> dict:
    return {"isDraft": False, "headRefOid": SHA, "number": 9, "headRefName": "main",
            "body": body, "labels": [{"name": n} for n in labels],
            "author": {"login": author},
            "reviewed": {"files": files, "comments": list(comments)}}


def verdict(p: dict) -> list[str]:
    return reviewed.refusal(p["reviewed"], p)


class Triggers(unittest.TestCase):
    def test_a_money_change_without_a_review_is_handed_back(self):
        ok, lines = mergeable.judge(pull(changed(MONEY)), GREEN, JOBS)
        self.assertFalse(ok)
        self.assertIn("money", lines[0])
        self.assertIn("`high`", lines[0])

    def test_an_untriggered_change_needs_nothing(self):
        ok, lines = mergeable.judge(pull(changed("crates/core/src/text/mod.rs")), GREEN, JOBS)
        self.assertTrue(ok, lines)

    def test_markdown_and_tests_never_trigger(self):
        files = changed("crates/server/src/credits/tests/ledger.rs",
                        ".github/scripts/tests/test_x.py", ".claude/skills/ci-merge/SKILL.md",
                        "crates/server/src/jobs/tests.rs")
        self.assertEqual(verdict(pull(files)), [])

    def test_the_highest_level_hit_is_the_one_asked_for(self):
        lines = verdict(pull(changed(JOBS_RS, MONEY)))
        self.assertIn("`high`", lines[0])
        self.assertIn("concurrency, money", lines[0])

    def test_the_gates_trigger_at_medium(self):
        lines = verdict(pull(changed(".github/workflows/ci.yml")))
        self.assertIn("the gates", lines[0])
        self.assertIn("`medium`", lines[0])

    def test_the_web_app_update_triggers_security(self):
        for path in ("deploy/update", "deploy/updating/cli.py"):
            self.assertIn("security", verdict(pull(changed(path)))[0], path)

    def test_a_rename_out_of_a_trigger_folder_still_counts(self):
        moved = [{"filename": "crates/providers/src/x.rs", "status": "renamed",
                  "previous_filename": MONEY}]
        self.assertIn("money", verdict(pull(moved))[0])

    def test_every_pattern_matches_a_tracked_path(self):
        tracked = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True,
                                 text=True, check=True).stdout.splitlines()
        dead = [p for _, _, patterns in reviewed.TRIGGERS for p in patterns
                if not any(fnmatch.fnmatchcase(path, p) for path in tracked)]
        self.assertEqual(dead, [], "a trigger nothing matches gates nothing")


class Recorded(unittest.TestCase):
    def test_a_comment_at_the_level_passes(self):
        note = "## Independent review — high\n\n- none found."
        self.assertEqual(verdict(pull(changed(MONEY), [note])), [])

    def test_a_higher_level_than_asked_passes(self):
        note = "**Independent review** (high)\nNothing to fix."
        self.assertEqual(verdict(pull(changed(JOBS_RS), [note])), [])

    def test_a_lower_level_than_asked_is_handed_back(self):
        lines = verdict(pull(changed(MONEY), ["## Independent review — medium"]))
        self.assertIn("ran at `medium`", lines[0])

    def test_a_comment_without_a_level_does_not_count(self):
        self.assertTrue(verdict(pull(changed(JOBS_RS), ["## Independent review\nok"])))

    def test_the_heading_has_to_open_the_comment(self):
        note = "Thanks!\n\n## Independent review — high"
        self.assertTrue(verdict(pull(changed(MONEY), [note])))

    def test_a_reviewer_is_not_a_review(self):
        self.assertTrue(verdict(pull(changed(MONEY), ["Independent reviewer: high"])))


class Excused(unittest.TestCase):
    def test_an_exception_with_a_reason_passes(self):
        body = "Gates: green\n\nNo independent review: only a doc comment changed.\n"
        self.assertEqual(verdict(pull(changed(MONEY), body=body)), [])

    def test_an_exception_without_a_reason_does_not(self):
        body = "Gates: green\nNo independent review: n/a\n"
        self.assertTrue(verdict(pull(changed(MONEY), body=body)))

    def test_dependabot_is_exempt_and_no_label_is(self):
        workflow = changed(".github/workflows/ci.yml")
        self.assertEqual(verdict(pull(workflow, author="app/dependabot")), [])
        self.assertTrue(verdict(pull(changed(MONEY), labels=["minor"])))


class Evidence(unittest.TestCase):
    def test_comments_are_asked_for_only_when_a_trigger_hits(self):
        asked = []

        def gh(*args):
            asked.append(args)
            return [{"body": "## Independent review — high"}]

        quiet = reviewed.evidence(gh, "o/r", pull([]), changed("web/src/App.tsx"))
        self.assertEqual((quiet["comments"], asked), ([], []))
        loud = reviewed.evidence(gh, "o/r", pull([]), changed(MONEY))
        self.assertEqual(loud["comments"], ["## Independent review — high"])
        self.assertIn("repos/o/r/issues/9/comments", asked[0][1])


if __name__ == "__main__":
    unittest.main()
