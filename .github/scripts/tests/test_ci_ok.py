#!/usr/bin/env python3
"""`CI ok` reports on every ready run and refuses whatever any job refused (#1351).

Two halves. `ci-ok.py`'s verdict over `toJSON(needs)`, and the shape of
`ci.yml` that makes the verdict mean something: no workflow-level `paths:`
(so every pull request gets a run), `CI ok` named exactly what the ruleset
requires and needing every other job, under `always()`, and every heavy job
gated on `changes`. A job left out of `CI ok`'s `needs:` is a job `main` no
longer waits for, which is the mistake this holds.
"""

from __future__ import annotations

import importlib.util
import os
import re
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

_spec = importlib.util.spec_from_file_location("ci_ok", ROOT / ".github" / "scripts" / "ci-ok.py")
ci_ok = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ci_ok)

# Jobs that run for a Markdown-only change too: `scripts` costs seconds, and
# its consistency tests read pages (#1226).
EVERY_CHANGE = ("scripts",)

DRAFT = "github.event_name != 'pull_request' || github.event.pull_request.draft == false"


def jobs(workflow: str) -> dict[str, str]:
    """Each job's block under `jobs:`, by id. Comments are kept out."""
    body = workflow.split("\njobs:\n", 1)[1]
    found: dict[str, list[str]] = {}
    current = None
    for line in body.splitlines():
        head = re.match(r"^  ([\w-]+):\s*$", line)
        if head:
            current = head.group(1)
            found[current] = []
        elif current and not line.strip().startswith("#"):
            found[current].append(line)
    return {name: "\n".join(lines) for name, lines in found.items()}


def needs(block: str) -> set[str]:
    match = re.search(r"^    needs: \[?([^\]\n]*)\]?$", block, re.M)
    return {n.strip() for n in match.group(1).split(",")} if match else set()


class Verdict(unittest.TestCase):
    def test_success_and_skipped_pass(self):
        self.assertEqual(ci_ok.refused({"a": {"result": "success"}, "b": {"result": "skipped"}}), [])

    def test_failure_and_cancelled_refuse(self):
        got = ci_ok.refused({"a": {"result": "failure"}, "b": {"result": "cancelled"}, "c": {"result": "success"}})
        self.assertEqual(got, ["a: failure", "b: cancelled"])

    def test_an_unknown_result_refuses(self):
        self.assertEqual(ci_ok.refused({"a": {}}), ["a: None"])

    def test_needing_nothing_refuses(self):
        self.assertTrue(ci_ok.refused({}))

    def test_main_exits_with_the_verdict(self):
        for needs_json, status in (('{"a": {"result": "skipped"}}', 0), ('{"a": {"result": "failure"}}', 1)):
            with mock.patch.dict(os.environ, {"NEEDS": needs_json}), mock.patch("builtins.print"):
                self.assertEqual(ci_ok.main(), status)


class Workflow(unittest.TestCase):
    def setUp(self) -> None:
        self.jobs = jobs(WORKFLOW)

    def test_every_pull_request_gets_a_run(self):
        triggers = WORKFLOW.split("\non:\n", 1)[1].split("\n\n", 1)[0]
        self.assertNotIn("paths", triggers, "a `paths:` filter makes no run, and `CI ok` never reports")
        self.assertIn("ready_for_review", triggers)

    def test_ci_ok_needs_every_other_job(self):
        ok = self.jobs["ok"]
        self.assertRegex(ok, r"(?m)^    name: CI ok$", "the ruleset requires this exact context")
        self.assertEqual(needs(ok), set(self.jobs) - {"ok"})
        self.assertIn(f"if: always() && ({DRAFT})", ok)
        self.assertIn("ci-ok.py", ok)

    def test_every_heavy_job_waits_for_changes(self):
        for name, block in self.jobs.items():
            if name in ("changes", "ok"):
                continue
            with self.subTest(job=name):
                self.assertEqual(needs(block), {"changes"})
                if name in EVERY_CHANGE:
                    self.assertIn(f"if: {DRAFT}\n", block)
                else:
                    self.assertIn(f"if: ({DRAFT}) && needs.changes.outputs.code == 'true'", block)

    def test_changes_asks_the_list_and_stays_silent_on_drafts(self):
        changes = self.jobs["changes"]
        self.assertIn(f"if: {DRAFT}", changes)
        self.assertIn("--no-renames", changes)
        self.assertIn(".github/scripts/ci-paths.py", changes)


if __name__ == "__main__":
    unittest.main()
