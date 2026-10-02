#!/usr/bin/env python3
"""`make mutants-remote` never reports an absence as a clean result.

The three ways it could, each held here on the pure decision it is made by:
reporting on code GitHub does not have (a head that was not pushed), reporting
on a run that is not the one it started (found by the request id in the run's
name, or not at all), and reading a red, cancelled or report-less run as zero
survivors. The second file in this pair, `mutants-diffs.py`, is held to its
own version of the last: a survivor without a diff is named, not dropped.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("mutants_remote", HERE / "mutants-remote.py")
assert _spec and _spec.loader
remote = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(remote)

SHA = "a" * 40


class OnlyWhatGitHubHas(unittest.TestCase):
    def test_the_pushed_head_goes_ahead(self) -> None:
        self.assertIsNone(remote.unpushed("651-x", SHA, SHA))

    def test_an_unpushed_commit_is_refused(self) -> None:
        said = remote.unpushed("651-x", SHA, "b" * 40)
        self.assertIn("push", said)

    def test_a_branch_github_never_saw_is_refused(self) -> None:
        self.assertIn("does not exist", remote.unpushed("651-x", SHA, None))

    def test_a_detached_head_is_refused(self) -> None:
        self.assertIn("detached", remote.unpushed("HEAD", SHA, SHA))


class ItsOwnRun(unittest.TestCase):
    RUNS = [
        {"displayTitle": "mutants: diff [1111aaaa]", "databaseId": 1},
        {"displayTitle": "mutants: diff [2222bbbb]", "databaseId": 2},
    ]

    def test_found_by_the_request_id(self) -> None:
        self.assertEqual(remote.find(self.RUNS, "2222bbbb")["databaseId"], 2)

    def test_another_request_on_the_same_branch_is_not_it(self) -> None:
        self.assertIsNone(remote.find(self.RUNS, "3333cccc"))


class NoReportIsNotZeroSurvivors(unittest.TestCase):
    def test_a_green_run_with_its_report_is_read(self) -> None:
        self.assertTrue(remote.verdict({"conclusion": "success", "url": "u"}, True)[0])

    def test_red_and_cancelled_runs_say_so(self) -> None:
        for conclusion in ("failure", "cancelled", "timed_out"):
            ok, said = remote.verdict({"conclusion": conclusion, "url": "u"}, False)
            self.assertFalse(ok)
            self.assertIn(conclusion, said)
            self.assertIn("not a clean result", said)

    def test_a_green_run_without_its_artifact_is_no_answer(self) -> None:
        ok, said = remote.verdict({"conclusion": "success", "url": "u"}, False)
        self.assertFalse(ok)
        self.assertIn("no mutants-report", said)


def missed(name: str, file: str, line: int) -> dict:
    span = {"start": {"line": line, "column": 1}, "end": {"line": line, "column": 2}}
    return {"scenario": {"Mutant": {"name": name, "file": file, "span": span}},
            "summary": "MissedMutant"}


class SurvivorsDiffs(unittest.TestCase):
    def render(self, outcomes: list[dict], planned: list[dict] | str) -> str:
        with tempfile.TemporaryDirectory() as tmp:
            run, plan = Path(tmp) / "run.json", Path(tmp) / "planned.json"
            run.write_text(json.dumps({"outcomes": outcomes}))
            plan.write_text(planned if isinstance(planned, str) else json.dumps(planned))
            done = subprocess.run(
                [sys.executable, str(HERE / "mutants-diffs.py"), str(run), str(plan)],
                capture_output=True, text=True, check=True,
            )
        return done.stdout

    def test_each_survivor_carries_its_diff_in_source_order(self) -> None:
        out = self.render(
            [missed("b.rs:9:1: y", "b.rs", 9), missed("a.rs:3:1: x", "a.rs", 3),
             {"scenario": "Baseline", "summary": "Success"}],
            [{"name": "a.rs:3:1: x", "diff": "-x\n+y\n"}, {"name": "b.rs:9:1: y", "diff": "-p\n+q\n"}],
        )
        self.assertLess(out.index("# a.rs:3:1: x"), out.index("# b.rs:9:1: y"))
        self.assertIn("-x\n+y", out)

    def test_a_survivor_the_plan_never_listed_is_named_anyway(self) -> None:
        out = self.render([missed("a.rs:3:1: x", "a.rs", 3)], "")
        self.assertIn("# a.rs:3:1: x", out)
        self.assertIn("no diff kept", out)

    def test_no_survivors_is_an_empty_file(self) -> None:
        self.assertEqual(self.render([], []), "")


if __name__ == "__main__":
    unittest.main()
