"""Waiting on CI: polling a pushed head until its run settles.

The evidence is read here and judged in [`verdict`]; the one retry a run
nobody judged earns is spent here too.
"""

from __future__ import annotations

import subprocess
import time

from . import mergeable, talk, verdict
from .states import GO, LATE, RERUN, STOP, UNJUDGED, WAIT


def evidence(repo: str, sha: str) -> tuple[list[dict], dict[int, list[dict]]]:
    """Every gating run for `sha`, with each one's jobs.

    Jobs are read per run rather than from `gh pr checks`, which blends runs —
    the blend is how a skipped run hides behind a real one (#245).
    """
    listed = mergeable.gh(
        "api", f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100"
    )
    runs = mergeable.runs_for(listed.get("workflow_runs", []), sha)
    jobs = {
        run["id"]: mergeable.gh(
            "api", f"repos/{repo}/actions/runs/{run['id']}/jobs"
        ).get("jobs", [])
        for run in runs
    }
    return runs, jobs


def wait_for(
    repo: str,
    number: int,
    sha: str,
    before: str,
    deadline: float,
    poll: float,
    tick=None,
) -> tuple[str, list[str]]:
    """Poll until the run on `sha` settles, or the deadline says stop.

    `tick`, when given, is called once a poll: the watch hears what changed on
    the board while one branch waits (#819).

    Never treats an absent check as a settled one — that is the trap the
    `ci-merge` skill names, and [`progress`] is where the two are told apart.
    [`head_state`] is the same distinction one level up: GitHub's view of the
    push lags the push.
    """
    began = time.monotonic()
    rerun: set[int] = set()
    # Asked once, the first time the head has no run: a Markdown-only diff
    # stays one however often it is polled (#1135).
    skips: bool | None = None
    while True:
        pull = talk.look(number)
        waited = time.monotonic() - began
        state, lines = verdict.head_state(
            pull.get("headRefOid", ""), sha, before, waited
        )
        if state == STOP:
            return state, lines
        if state == GO:
            # Read every poll: the label that counts is the one at merge time.
            pull["closes"] = mergeable.closing.closed_by(mergeable.gh, repo, pull)
            runs, jobs = evidence(repo, sha)
            if not runs and skips is None:
                skips = mergeable.skipped_by_ci(
                    repo, number, pull.get("baseRefName")
                )
            state, lines = verdict.progress(
                pull, runs, jobs, waited, frozenset(rerun), bool(skips)
            )
            if state == RERUN:
                run = verdict.never_judged(runs, jobs)[0]
                state, lines = rerun_failed(
                    run, [f"#{number}: {lines[0]}", *lines[1:]]
                )
                if state != WAIT:
                    return state, lines
                rerun.add(run["id"])
            elif state != WAIT:
                return state, lines
        if waited > deadline:
            return LATE, [
                f"still waiting on {sha[:7]} after {deadline / 60:.0f} minutes.",
                *lines,
                "Unfinished, with the run still out. Nothing is red; nothing is"
                " green either. Run the queue again to pick it back up.",
            ]
        talk.say(f"#{number}: {lines[0]}")
        if tick is not None:
            tick()
        time.sleep(poll)


def rerun_failed(run: dict, lines: list[str]) -> tuple[str, list[str]]:
    """Re-run `run`'s failed jobs, the one retry [`not_judged`] allows.

    A refusal from GitHub is not a verdict on the code either, so it ends the
    turn unfinished, with GitHub's reason, rather than red.
    """
    talk.say(*lines)
    done = subprocess.run(
        ["gh", "run", "rerun", str(run["id"]), "--failed"],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        return UNJUDGED, [
            f"the re-run of {run.get('html_url', 'the run')} was refused:"
            f" {done.stderr.strip()}",
            "Nothing is red; nothing is green either. Re-run its failed jobs"
            " and queue it again.",
        ]
    return WAIT, [f"re-ran the failed jobs of {run.get('html_url', 'the run')}."]
