"""The description's protocol (#1229): a branch named for an issue closes it,
and a missing Gates line is said out loud.

`protocol.py` is pure, and `mergeable.judge` asks it only once CI has passed,
so both are tested by import with the same fixtures as `test_mergeable.py`.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "mergeable.py"
_spec = importlib.util.spec_from_file_location("mergeable", SCRIPT)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)
protocol = mergeable.protocol

SHA = "c23b7c4f0000000000000000000000000000000a"
GREEN = [{"name": "CI", "head_sha": SHA, "id": 1, "status": "completed",
          "conclusion": "success", "html_url": "https://example.invalid/1"}]
JOBS = {1: [{"name": "fmt + clippy + test", "conclusion": "success"}]}
GOOD = "Closes #1229\n\nThe change.\n\n**Gates**: `make gates` green on abc1234.\n"


def pull(branch: str = "1229-small-enforcement", body: str = GOOD) -> dict:
    return {"isDraft": False, "headRefOid": SHA, "number": 1282,
            "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN",
            "headRefName": branch, "body": body}


class Closing(unittest.TestCase):
    def test_a_branch_whose_description_closes_its_issue_passes(self):
        ok, lines = mergeable.judge(pull(), GREEN, JOBS)
        self.assertTrue(ok, lines)
        self.assertFalse(any("Warning" in line for line in lines), lines)

    def test_a_branch_whose_description_forgets_its_issue_is_handed_back(self):
        ok, lines = mergeable.judge(pull(body="Refs the batch.\n\nGates: green"), GREEN, JOBS)
        self.assertFalse(ok)
        self.assertIn("does not close #1229", lines[0])
        self.assertIn("Closes #1229", lines[1])

    def test_another_issue_number_is_not_this_one(self):
        self.assertFalse(protocol.names("Closes #12290 and fixes #229", 1229))
        self.assertEqual(len(protocol.closing(pull(body="Closes #12290"))), 2)

    def test_every_keyword_github_honours_and_refs_count(self):
        for body in ("closes #7", "Fixes: #7", "RESOLVED #7", "fix #7", "refs #7"):
            self.assertTrue(protocol.names(body, 7), body)
        self.assertFalse(protocol.names("see #7", 7))

    def test_a_branch_named_for_no_issue_is_not_asked(self):
        self.assertEqual(protocol.closing(pull(branch="dependabot/cargo/clap-4.7", body="")), [])
        self.assertEqual(protocol.issue_of("main"), None)

    def test_a_red_run_is_still_what_a_refusal_names(self):
        red = [GREEN[0] | {"conclusion": "failure"}]
        ok, lines = mergeable.judge(pull(body=""), red, JOBS)
        self.assertFalse(ok)
        self.assertIn("concluded failure", lines[0])

    def test_a_pull_read_without_the_fields_is_not_judged_on_them(self):
        bare = {key: value for key, value in pull(body="").items()
                if key not in ("body", "headRefName")}
        ok, lines = mergeable.judge(bare, GREEN, JOBS)
        self.assertTrue(ok, lines)
        self.assertEqual(protocol.gates(bare), [])


class Gates(unittest.TestCase):
    def test_a_missing_gates_line_is_a_warning_not_a_refusal(self):
        ok, lines = mergeable.judge(pull(body="Closes #1229\n\nNo report."), GREEN, JOBS)
        self.assertTrue(ok, lines)
        self.assertIn("no **Gates** line", lines[-1])

    def test_every_way_the_line_is_written_counts(self):
        for body in ("Gates: green", "**Gates** — green", "## Gates\n- fmt",
                     "- **Gates**: green", "  Gates green on abc"):
            self.assertEqual(protocol.gates({"body": body}), [], body)
        self.assertEqual(len(protocol.gates({"body": "The gates were green"})), 1)


if __name__ == "__main__":
    unittest.main()
