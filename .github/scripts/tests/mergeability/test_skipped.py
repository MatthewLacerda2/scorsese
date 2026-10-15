#!/usr/bin/env python3
"""A Markdown-only pull request gets a run and is waited on like any other (#1351).

Before #1351, `ci.yml`'s `paths:` filter made no run for a Markdown-only change
and `mergeable` said yes to that absence (#1135). Now the `changes` job asks
`ci-paths.py` instead and every heavy job skips, so the run that comes back is
`changes` and `CI ok` passing with the rest skipped. That is a yes; no run at
all is #153 whatever the diff. Nothing here talks to `gh`.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[2]


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ci_paths = load("ci_paths", "ci-paths.py")
mergeable = load("mergeable", "mergeable.py")
queue = load("merge_queue", "merge-queue.py")

SHA = "c23b7c4f0000000000000000000000000000000a"
READY = {
    "isDraft": False,
    "headRefOid": SHA,
    "number": 1351,
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
    "headRefName": "docs-only",
    "body": "A doc fix.\n\n**Gates**: `make scripts`.",
}
RUN = {"id": 1, "name": "CI", "head_sha": SHA, "status": "completed", "conclusion": "success"}
HEAVY = ("fmt + clippy + test", "desktop app", "web front-end", "size gate")


def job(name: str, conclusion: str) -> dict:
    return {"name": name, "conclusion": conclusion}


# What a Markdown-only run reports: two jobs that ran, every other one skipped.
MARKDOWN_RUN = {
    1: [job("changes", "success"), *(job(n, "skipped") for n in HEAVY), job("CI ok", "success")]
}


class Filter(unittest.TestCase):
    def code(self, *files: str) -> str:
        return ci_paths.main([f"{name}\n" for name in files])

    def test_a_markdown_only_diff_runs_no_job(self):
        files = ("CLAUDE.md", ".claude/skills/ci-merge/SKILL.md", "crates/core/CLAUDE.md")
        self.assertEqual(self.code(*files), "code=false")

    def test_a_page_the_code_reads_runs_them(self):
        self.assertEqual(self.code("docs/pages.md"), "code=true")
        self.assertEqual(self.code("README.md", "docs/pages.md"), "code=true")

    def test_any_file_that_is_not_markdown_runs_them(self):
        self.assertEqual(self.code("README.md", "Makefile"), "code=true")
        self.assertEqual(self.code("crates/core/src/lib.rs"), "code=true")

    def test_an_empty_or_unread_diff_runs_them(self):
        self.assertEqual(self.code(), "code=true")
        self.assertEqual(ci_paths.main(["\n", "  \n"]), "code=true")

    def test_the_last_matching_pattern_decides(self):
        patterns = ["**", "!**.md", "docs/*.md"]
        self.assertTrue(ci_paths.runs_for(patterns, ["docs/a.md"]))
        # `*` stops at a slash, as it does in GitHub's filter.
        self.assertFalse(ci_paths.runs_for(patterns, ["docs/deep/a.md"]))
        self.assertTrue(ci_paths.runs_for([], ["a.md"]), "no filter runs everything")


class Judge(unittest.TestCase):
    def test_a_markdown_only_run_is_yes(self):
        ok, lines = mergeable.judge(READY, [RUN], MARKDOWN_RUN)
        self.assertTrue(ok, lines)
        self.assertIn("2 jobs ran", lines[0])
        self.assertIn("fmt + clippy + test", lines[1])

    def test_no_run_is_no_whatever_the_diff(self):
        ok, lines = mergeable.judge(READY, [], {})
        self.assertFalse(ok)
        self.assertIn("#153", " ".join(lines))

    def test_a_draft_run_that_skipped_everything_is_still_no(self):
        draft = {1: [job(n, "skipped") for n in ("changes", *HEAVY, "CI ok")]}
        ok, lines = mergeable.judge(READY, [RUN], draft)
        self.assertFalse(ok)
        self.assertIn("ran a single job", lines[0])

    def test_the_old_answer_is_gone(self):
        self.assertFalse(hasattr(mergeable, "skipped_by_ci"))


class Queue(unittest.TestCase):
    def test_no_run_yet_is_waited_on(self):
        state, lines = queue.progress(READY, [], {}, 0.0)
        self.assertEqual(state, queue.WAIT, lines)

    def test_no_run_after_the_grace_stops_it(self):
        state, lines = queue.progress(READY, [], {}, queue.RUN_APPEARS_SECONDS + 1)
        self.assertEqual(state, queue.STOP)
        self.assertIn("#153", " ".join(lines))

    def test_a_markdown_only_run_merges(self):
        state, lines = queue.progress(READY, [RUN], MARKDOWN_RUN, 30.0)
        self.assertEqual(state, queue.GO, lines)


if __name__ == "__main__":
    unittest.main()
