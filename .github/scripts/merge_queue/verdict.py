"""Whether to wait, merge or hand back, from one poll's evidence. Pure.

The queue's whole decision, taken apart from the polling in [`ci`] so it is
tested on plain dictionaries: *not yet* against *no*, a head GitHub has not
caught up with, and a run nobody judged (#792).
"""

from __future__ import annotations

from . import mergeable
from .states import GO, RERUN, STOP, UNJUDGED, WAIT


# How long "there is no run yet" is read as *not yet* rather than as an answer.
# A force-push and the run it creates are not simultaneous — GitHub has to
# evaluate the workflow's triggers first — so reading the gap as #153 would
# refuse every branch this script pushes. After this, the silence is the
# answer, and `mergeable.no_run` says which of #153 and #429 it is.
RUN_APPEARS_SECONDS = 300


def head_state(
    seen: str, pushed: str, before: str, waited: float
) -> tuple[str, list[str]]:
    """Whether the head GitHub reports is the one this queue put there.

    Three answers, and the middle one is why this is a function. A force-push
    and GitHub's view of it are not simultaneous, so the first poll after one
    routinely still reports `before` — the pre-push head. Reading that as
    *somebody else pushed* would hand back nearly every branch this script
    touches, and reading any other sha as *fine* would let it merge a commit it
    never watched. So: the pushed head goes, the pre-push head waits while the
    grace lasts, and a third sha is somebody else and stops.
    """
    if seen == pushed:
        return GO, []
    if seen == before and waited < RUN_APPEARS_SECONDS:
        return WAIT, [
            f"GitHub still reports {before[:7]}; it has not caught up with the"
            " push yet."
        ]
    return STOP, [
        f"the head is {seen[:7]}, not the {pushed[:7]} this queue pushed.",
        "Somebody else pushed. Handed back rather than merging a commit this"
        " queue never watched.",
    ]


def unstarted(job: dict) -> bool:
    """A job GitHub cancelled before a single one of its steps completed.

    What a job looks like when no runner took it in time: `cancelled`, and
    `steps` empty — every such job of run 37367713417 on #788 (#792). A step
    that completed means a runner did start it, and a job cancelled after that
    may have been cancelled *by* what it found — a timeout is a verdict too —
    so it is not this, and stays red.
    """
    return job.get("conclusion") == "cancelled" and not any(
        step.get("status") == "completed" for step in job.get("steps") or []
    )


def never_judged(runs: list[dict], jobs: dict[int, list[dict]]) -> list[dict]:
    """The failed runs, if **every** one of them failed without judging anything.

    A run is not judged when it has at least one job that did not pass and
    every such job is [`unstarted`]: the jobs that ran passed, and the rest
    never ran. One job that genuinely failed, in any failed run, empties the
    answer, because a red verdict anywhere is still the answer for the commit
    — the out-of-scope line of #792 is *red stays red*.
    """
    failed = mergeable.failed_runs(runs)

    def unjudged(run: dict) -> bool:
        bad = [
            job
            for job in jobs.get(run.get("id"), [])
            if job.get("conclusion") not in ("success", "skipped")
        ]
        return bool(bad) and all(unstarted(job) for job in bad)

    return failed if failed and all(unjudged(run) for run in failed) else []


def progress(
    pull: dict,
    runs: list[dict],
    jobs: dict[int, list[dict]],
    waited: float,
    rerun: frozenset[int] = frozenset(),
    skips: bool = False,
) -> tuple[str, list[str]]:
    """Whether to wait, merge, or hand this branch back — and why.

    Pure, and the whole of the queue's decision. The verdict itself is
    [`mergeable.judge`]; what this adds is the distinction that a polling loop
    needs and a one-shot check does not: *not yet* against *no*.

    Order matters and mirrors `judge`'s. A failure is asked about before a run
    in flight, because a commit carrying both a red run and a live one is
    already answered — waiting out the live one would sit through ten minutes
    to be told what the red one said at the start.

    The `waited` grace is the other half, and it covers **absence of evidence**
    rather than only an absent run. `judge` is right to call both no-run and
    every-run-skipped a refusal: it is asked once, of a commit that has been
    sitting there. Here the commit was pushed seconds ago, and neither shape is
    an answer yet.

    That the grace has to cover the second shape and not only the first was
    learned from this endpoint on 2026-08-29, watching this very branch:
    `actions/runs?head_sha=…` returned the commit's skipped draft run and
    *omitted its live one*, seconds after `gh run list` had shown both. Read
    literally that poll says "nothing built this commit" — which is exactly
    what a run against a draft says, and would have handed back a branch that
    was three minutes from green. Absence is eventually-consistent; a red run
    is not, which is why only red stops inside the grace.

    `skips` is [`mergeable.skipped_by_ci`]: `ci.yml` makes no run for this
    diff by design (#1135). Then there is no run to give a grace to, and what
    is left is GitHub's mergeability, waited on only while it reads UNKNOWN.
    """
    sha = pull.get("headRefOid", "")
    short = sha[:7]

    if pull.get("isDraft"):
        # Asked before anything is pushed, too — see `take`. A draft asserts
        # nothing, so there is no claim here to check.
        return STOP, [
            "the pull request is a draft.",
            "CI does not run on drafts. Mark it ready before queueing it.",
        ]

    # What it closes is asked before CI, too: no run turns a `planning` issue
    # into one that may be started (#1221).
    refused = mergeable.closing.refusal(pull.get("closes") or {})
    if refused:
        return STOP, refused

    unjudged = never_judged(runs, jobs)
    if unjudged:
        return not_judged(unjudged[0], jobs, rerun, short)

    if mergeable.failed_runs(runs):
        return STOP, mergeable.judge(pull, runs, jobs)[1]

    live = mergeable.unfinished(runs)
    if live:
        return WAIT, [
            f"a {mergeable.WORKFLOW} run for {short} is"
            f" {live[0].get('status')}."
        ]

    if not runs and skips:
        ok, lines = mergeable.judge(pull, runs, jobs, skips)
        if ok:
            return GO, lines
        return (WAIT if pull.get("mergeable") == mergeable.UNKNOWN else STOP), lines

    # Nothing has compiled this commit — either no run at all, or only runs
    # that skipped everything. Both are absences, and both get the grace.
    if not [run for run in runs if mergeable.ran_something(run, jobs)]:
        if waited < RUN_APPEARS_SECONDS:
            return WAIT, [
                f"nothing has built {short} yet.",
                "Ordinary this soon after a push, and not yet an answer.",
            ]
        if not runs:
            return STOP, mergeable.no_run(pull, short)

    ok, lines = mergeable.judge(pull, runs, jobs)
    return (GO if ok else STOP), lines


def not_judged(
    run: dict, jobs: dict[int, list[dict]], rerun: frozenset[int], short: str
) -> tuple[str, list[str]]:
    """What to do about a run [`never_judged`] found: re-run once, then stop.

    Once is counted two ways, and both are needed. `rerun` is the ids this
    turn already re-ran, so a poll that still shows the old attempt — GitHub's
    run record lags a re-run like it lags a push — waits instead of asking
    twice. `run_attempt` is GitHub's own count, so a watch restarted between
    the two attempts does not re-run a second time: an attempt past the first
    that was not judged either is handed on as unfinished, never as red.
    """
    names = ", ".join(
        f"`{job.get('name')}`"
        for job in jobs.get(run.get("id"), [])
        if unstarted(job)
    )
    url = run.get("html_url", "the run")
    if (run.get("run_attempt") or 1) > 1:
        return UNJUDGED, [
            f"a {mergeable.WORKFLOW} run for {short} went unjudged on attempt"
            f" {run.get('run_attempt')} too: {names} cancelled before a step ran.",
            f"See {url}.",
            "Nothing is red; nothing is green either. Re-run its failed jobs"
            " and queue it again.",
        ]
    if run.get("id") in rerun:
        return WAIT, [f"waiting for the re-run of {url} to start."]
    return RERUN, [
        f"a {mergeable.WORKFLOW} run for {short} concluded"
        f" {run.get('conclusion')} without judging it: {names} were cancelled"
        " before a single step ran.",
        f"No runner, not a verdict: re-running its failed jobs once ({url}).",
    ]
