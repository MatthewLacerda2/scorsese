#!/usr/bin/env python3
"""A Markdown-only pull request is mergeable without a run, and nothing else is (#1135).

`ci.yml` makes no run for a change its `paths:` filter skips, so `mergeable`
used to refuse every Markdown-only pull request and the skill kept a merge by
hand for them. The yes it gives now is narrow: no run, every changed file one
the filter skips (read by the parser `test_ci_read_docs.py` uses), and GitHub
calling the branch mergeable. A page the code reads is re-included, so a change
to it with no run is still #153.

Held against the real `ci.yml`, so a change to its filter is a change these
answers see. Nothing here talks to `gh`.
"""

from __future__ import annotations

import base64
import importlib.util
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
WORKFLOW = (SCRIPTS.parent / "workflows" / "ci.yml").read_text(encoding="utf-8")


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
    "number": 1135,
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
}


class Filter(unittest.TestCase):
    def test_a_markdown_only_diff_is_skipped(self):
        files = ["CLAUDE.md", ".claude/skills/ci-merge/SKILL.md", "crates/core/CLAUDE.md"]
        self.assertTrue(ci_paths.skips(WORKFLOW, files))

    def test_a_page_the_code_reads_is_not(self):
        self.assertFalse(ci_paths.skips(WORKFLOW, ["docs/pages.md"]))
        self.assertFalse(ci_paths.skips(WORKFLOW, ["README.md", "docs/pages.md"]))

    def test_any_file_that_is_not_markdown_is_not(self):
        self.assertFalse(ci_paths.skips(WORKFLOW, ["README.md", "Makefile"]))
        self.assertFalse(ci_paths.skips(WORKFLOW, ["crates/core/src/lib.rs"]))

    def test_an_empty_diff_is_not(self):
        self.assertFalse(ci_paths.skips(WORKFLOW, []))

    def test_a_rename_counts_both_ends(self):
        listing = [{"filename": "notes.md", "previous_filename": "notes.rs"}]
        self.assertEqual(mergeable.changed(listing), ["notes.md", "notes.rs"])
        self.assertFalse(ci_paths.skips(WORKFLOW, mergeable.changed(listing)))

    def test_the_last_matching_pattern_decides(self):
        patterns = ["**", "!**.md", "docs/*.md"]
        self.assertTrue(ci_paths.runs_for(patterns, ["docs/a.md"]))
        # `*` stops at a slash, as it does in GitHub's filter.
        self.assertFalse(ci_paths.runs_for(patterns, ["docs/deep/a.md"]))
        self.assertTrue(ci_paths.runs_for([], ["a.md"]), "no filter runs everything")


class Judge(unittest.TestCase):
    def test_skipped_and_mergeable_is_yes(self):
        ok, lines = mergeable.judge(READY, [], {}, skips=True)
        self.assertTrue(ok, lines)

    def test_no_run_without_the_filter_is_still_no(self):
        ok, lines = mergeable.judge(READY, [], {})
        self.assertFalse(ok)
        self.assertIn("#153", " ".join(lines))

    def test_skipped_but_conflicted_is_no(self):
        pull = READY | {"mergeable": "CONFLICTING", "mergeStateStatus": "DIRTY"}
        ok, lines = mergeable.judge(pull, [], {}, skips=True)
        self.assertFalse(ok)
        self.assertIn("#429", " ".join(lines))

    def test_skipped_but_not_yet_computed_is_no(self):
        ok, lines = mergeable.judge(READY | {"mergeable": "UNKNOWN"}, [], {}, skips=True)
        self.assertFalse(ok)
        self.assertIn("Ask again", " ".join(lines))

    def test_a_draft_is_no_even_when_skipped(self):
        self.assertFalse(mergeable.judge(READY | {"isDraft": True}, [], {}, skips=True)[0])


class Asking(unittest.TestCase):
    def ask(self, listing: list[dict]):
        found = {"content": base64.b64encode(WORKFLOW.encode()).decode()}
        with mock.patch.object(mergeable, "gh", side_effect=[listing, found]) as gh:
            return mergeable.skipped_by_ci("o/r", 1135, "main"), gh

    def test_a_markdown_listing_is_skipped(self):
        skipped, gh = self.ask([{"filename": "CLAUDE.md"}])
        self.assertTrue(skipped)
        self.assertIn("ref=main", gh.call_args_list[1].args[1])

    def test_a_listing_that_may_be_cut_short_is_not(self):
        skipped, gh = self.ask([{"filename": f"{n}.md"} for n in range(mergeable.FILES_PAGE)])
        self.assertFalse(skipped)
        self.assertEqual(gh.call_count, 1, "no workflow read for a listing it cannot trust")


class Queue(unittest.TestCase):
    def test_it_merges_without_waiting_out_the_grace(self):
        state, lines = queue.progress(READY, [], {}, 0.0, skips=True)
        self.assertEqual(state, queue.GO, lines)

    def test_it_waits_while_github_computes_mergeability(self):
        state, _ = queue.progress(READY | {"mergeable": "UNKNOWN"}, [], {}, 0.0, skips=True)
        self.assertEqual(state, queue.WAIT)

    def test_a_conflict_stops_it(self):
        pull = READY | {"mergeable": "CONFLICTING"}
        self.assertEqual(queue.progress(pull, [], {}, 0.0, skips=True)[0], queue.STOP)


if __name__ == "__main__":
    unittest.main()
