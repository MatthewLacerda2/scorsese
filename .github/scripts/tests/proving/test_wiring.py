"""`ci.yml` wires `proved_tree.py` the way its skip relies on (#1758).

Text checks over the workflow: the job and step names `BUILT` reads exist, the
`changes` job asks only on a push and folds the answer into `code` and
`proved`, a broken lookup cannot fail `changes`, every job a proved push skips
is one `BUILT` checks or one that has nothing to build, and the nightly that
rebuilds `main` in full is there.
"""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

TESTS = Path(__file__).resolve().parents[1]
if str(TESTS) not in sys.path:
    sys.path.insert(0, str(TESTS))
if str(TESTS.parent) not in sys.path:
    sys.path.insert(0, str(TESTS.parent))

import proved_tree  # noqa: E402
from test_ci_ok import WORKFLOW, jobs  # noqa: E402


def step(block: str, name: str) -> str:
    """The step called `name`, up to the next step."""
    start = block.index(f"- name: {name}\n")
    end = re.search(r"\n      - (uses|name):", block[start + 1:])
    return block[start:start + 1 + end.start()] if end else block[start:]


def named(blocks: dict[str, str]) -> dict[str, str]:
    """Each job's block by the `name:` the jobs API reports."""
    found = {}
    for block in blocks.values():
        match = re.search(r"^    name: (.+)$", block, re.M)
        if match:
            found[match.group(1)] = block
    return found


class Wiring(unittest.TestCase):
    def setUp(self) -> None:
        self.jobs = jobs(WORKFLOW)

    def test_built_names_match_the_workflow(self):
        by_name = named(self.jobs)
        for name, checking in proved_tree.BUILT.items():
            with self.subTest(job=name):
                self.assertIn(name, by_name, "a job `BUILT` names is gone or renamed")
                self.assertIn(f"- name: {checking}\n", by_name[name])

    def test_every_job_a_proved_push_skips_is_checked_by_built(self):
        """A job left out of `BUILT` would land on `main` with no run proving it."""
        by_name = named(self.jobs)
        skips = {name for name, block in by_name.items()
                 if "needs.changes.outputs.code == 'true'" in block
                 or "needs.changes.outputs.proved != 'true'" in block}
        self.assertEqual(skips - set(proved_tree.BUILT), {"regression test"},
                         "only `regression test`, which `main` never owes, may skip unproved")

    def test_changes_asks_and_folds_the_answer(self):
        changes = self.jobs["changes"]
        self.assertIn("proved_tree.py", changes)
        self.assertIn("code: ${{ steps.proved.outputs.proved == 'true' && 'false' || "
                      "steps.diff.outputs.code || 'true' }}", changes)
        self.assertIn("proved: ${{ steps.proved.outputs.proved == 'true' }}", changes)
        self.assertIn("actions: read", changes)

    def test_a_broken_lookup_fails_the_step_not_the_job(self):
        asks = step(self.jobs["changes"], "Was this tree already proved?")
        self.assertIn("if: github.event_name == 'push' && steps.diff.outputs.code != 'false'", asks)
        self.assertIn("continue-on-error: true", asks)
        self.assertRegex(asks, r"timeout-minutes: [1-4]\n")

    def test_the_nightly_rebuilds_main(self):
        triggers = WORKFLOW.split("\non:\n", 1)[1].split("\nenv:\n", 1)[0]
        self.assertRegex(triggers, r"\n  schedule:\n    - cron: '\d")


if __name__ == "__main__":
    unittest.main()
