#!/usr/bin/env python3
"""A run whose jobs never started is not red, and is re-run once (#792).

`fixtures/run-cancelled-unstarted.json` is attempt 1 of run 37367713417 on
#788, recorded with `gh api …/runs/37367713417/attempts/1/jobs` and trimmed to
the fields the queue reads: three jobs passed, four were cancelled with no step
run, and the run concluded `failure`. The queue handed it back as red; its
re-run came back all green. Every test here starts from that record, and the
ones that must stay red are that record with one job really failing.
"""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
SCRIPT = HERE.parent / "merge-queue.py"
FIXTURE = json.loads((HERE / "fixtures" / "run-cancelled-unstarted.json").read_text())

_spec = importlib.util.spec_from_file_location("merge_queue", SCRIPT)
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)

RUN = FIXTURE["run"]
SHA = RUN["head_sha"]
PULL = {"isDraft": False, "headRefOid": SHA, "number": 788, "state": "OPEN"}


def recorded(**fields: object) -> tuple[dict, list[dict]]:
    """The recorded run, with `fields` changed, and a copy of its jobs."""
    return RUN | fields, copy.deepcopy(FIXTURE["jobs"])


def asked(run: dict, jobs: list[dict], rerun=frozenset(), others=()):
    runs = [run, *others]
    table = {run["id"]: jobs} | {o["id"]: [] for o in others}
    return queue.progress(PULL, runs, table, 600.0, rerun)


def really_failed(jobs: list[dict], name: str = "fmt + clippy + test") -> None:
    for job in jobs:
        if job["name"] == name:
            job["conclusion"] = "failure"
            job["steps"][-1]["conclusion"] = "failure"


class NotJudged(unittest.TestCase):
    def test_the_recorded_run_is_re_run_and_not_handed_back(self):
        state, lines = asked(*recorded())
        self.assertEqual(state, queue.RERUN)
        for name in ("desktop app", "web front-end", "deploy config", "signal renderers"):
            self.assertIn(f"`{name}`", lines[0])
        self.assertIn(RUN["html_url"], lines[1])

    def test_a_re_run_already_asked_for_is_waited_on_not_asked_twice(self):
        # GitHub's run record lags a re-run; the stale poll must not re-run again.
        state, _ = asked(*recorded(), rerun=frozenset({RUN["id"]}))
        self.assertEqual(state, queue.WAIT)

    def test_a_second_attempt_that_went_unjudged_is_unfinished_not_red(self):
        state, lines = asked(*recorded(run_attempt=2))
        self.assertEqual(state, queue.UNJUDGED)
        self.assertIn("attempt 2", lines[0])
        self.assertIn(RUN["html_url"], " ".join(lines))

    def test_one_job_really_failing_keeps_it_red(self):
        run, jobs = recorded()
        really_failed(jobs)
        state, lines = asked(run, jobs)
        self.assertEqual(state, queue.STOP)
        self.assertIn("concluded failure", lines[0])

    def test_a_job_cancelled_after_a_step_ran_is_a_verdict(self):
        # A runner took it; a timeout mid-test is an answer, not an absence.
        run, jobs = recorded()
        cancelled = next(job for job in jobs if job["name"] == "desktop app")
        cancelled["steps"] = [
            {"name": "Set up job", "status": "completed", "conclusion": "success"}
        ]
        self.assertEqual(asked(run, jobs)[0], queue.STOP)

    def test_a_red_run_beside_an_unjudged_one_is_still_red(self):
        red = RUN | {"id": 2, "html_url": "https://example.invalid/run/2"}
        state, lines = asked(*recorded(), others=(red,))
        self.assertEqual(state, queue.STOP)
        self.assertIn("concluded failure", lines[0])

    def test_a_failed_run_with_no_jobs_listed_is_not_assumed_unjudged(self):
        self.assertEqual(asked(RUN, [])[0], queue.STOP)


class WaitFor(unittest.TestCase):
    """The loop around it: one re-run, then the new attempt is waited on."""

    def polled(self, attempts, rerun_code=0):
        evidence = iter(attempts)
        done = subprocess.CompletedProcess([], rerun_code, "", "not allowed")
        with mock.patch.object(queue, "look", return_value=PULL), mock.patch.object(
            queue, "evidence", side_effect=lambda *_: next(evidence)
        ), mock.patch.object(queue.subprocess, "run", return_value=done) as gh, \
                mock.patch.object(queue.time, "sleep"), mock.patch.object(queue, "say"):
            outcome = queue.wait_for("o/r", 788, SHA, SHA, 3600, 0)
        return outcome, gh

    def test_it_re_runs_once_and_then_judges_the_new_attempt(self):
        run, jobs = recorded()
        stale = ([run], {run["id"]: jobs})
        live = ([RUN | {"status": "in_progress", "run_attempt": 2}], {run["id"]: []})
        green_jobs = [job | {"conclusion": "success"} for job in jobs]
        green_jobs[0]["steps"] = [{"status": "completed"}]
        green = ([RUN | {"conclusion": "success", "run_attempt": 2}], {run["id"]: green_jobs})
        (state, _), gh = self.polled([stale, stale, live, green])
        self.assertEqual(state, queue.GO)
        gh.assert_called_once()
        self.assertEqual(gh.call_args.args[0], ["gh", "run", "rerun", str(RUN["id"]), "--failed"])

    def test_a_refused_re_run_is_unfinished_not_red(self):
        run, jobs = recorded()
        (state, lines), _ = self.polled([([run], {run["id"]: jobs})], rerun_code=1)
        self.assertEqual(state, queue.UNJUDGED)
        self.assertIn("not allowed", lines[0])


if __name__ == "__main__":
    unittest.main()
