"""The train on GitHub: its branch, its CI run, and whether `main` lets a train land (#1757).

`merge-train/*` branches are the queue's own throwaways: nobody else writes
them, one queue at a time holds the lock, and every one left behind is swept
when a watch starts ([`sweep`]). Pushing one is never a force: a branch of the
same name is deleted first. CI runs on it by `workflow_dispatch` ([`dispatch`]),
which runs every gating job (`changes` cannot diff a dispatch, so it runs
everything) and owes no regression test: each member's own run on its own head
answered that one.

[`wait`] is the queue's own reading of a run ([`verdict`]) with no pull request
to claim anything: a run not listed yet waits out its grace, a run nobody
judged is re-run once, and only a run that failed for the code's reasons is
[`RED`]. Anything else that ends the wait is [`STALLED`]: not the trees' fault.

[`landable`] asks the ruleset first: while `main` requires a branch to hold its
tip (`strict_required_status_checks_policy`), GitHub refuses every member but a
rebased first one, so the watch merges one at a time and says why.
"""

from __future__ import annotations

import subprocess
import time

from . import ci, mergeable, talk, train as trains, verdict
from .states import GO, RERUN, WAIT


# How a train's wait ends besides [`GO`]: its run said no, or nothing said
# anything in time. Either way the train is halved; only red marks its members.
RED, STALLED = "red", "stalled"

WORKFLOW_FILE = "ci.yml"


def landable(repo: str) -> str | None:
    """Why `main`'s ruleset keeps a train from landing, or `None` when it does not.

    Asked once a watch. An answer GitHub would not give is a reason too: the
    watch then merges one at a time, which is never wrong, only slower.
    """
    try:
        rules = mergeable.gh("api", f"repos/{repo}/rules/branches/main")
    except (mergeable.Unreachable, SystemExit) as down:
        return f"GitHub did not show `main`'s rules ({down})"
    checks = [r for r in rules or [] if r.get("type") == "required_status_checks"]
    if any((r.get("parameters") or {}).get("strict_required_status_checks_policy", True) for r in checks):
        return (
            "`main`'s ruleset requires a branch to hold its tip before it merges"
            " (`strict_required_status_checks_policy`), which no member of a train"
            " but the first can"
        )
    return None


def tip_state(runs: list[dict], jobs: dict[int, list[dict]], waited: float, rerun: frozenset[int]) -> tuple[str, list[str]]:
    """One poll's answer about the train's tip: wait, go, re-run, red or stalled. Pure."""
    unjudged = verdict.never_judged(runs, jobs)
    if unjudged:
        state, lines = verdict.not_judged(unjudged[0], jobs, rerun, "the train")
        return (STALLED if state not in (WAIT, RERUN) else state), lines
    failed = mergeable.failed_runs(runs)
    if failed:
        return RED, [
            f"the train's {mergeable.WORKFLOW} run concluded {failed[0].get('conclusion')}.",
            f"See {failed[0].get('html_url', 'the run')}.",
        ]
    live = mergeable.unfinished(runs, jobs)
    if live:
        return WAIT, [f"the train's {mergeable.WORKFLOW} run is {mergeable.pending(live[0], jobs)}."]
    if not [run for run in runs if mergeable.ran_something(run, jobs)]:
        if waited < verdict.RUN_APPEARS_SECONDS:
            return WAIT, ["nothing has built the train yet."]
        return STALLED, [f"no {mergeable.WORKFLOW} run built the train in {waited / 60:.0f} minutes."]
    unsummed = mergeable.unsummarised(runs, jobs, "the train")
    if unsummed:  # no member's own `CI ok` stands in for the train's (#1764)
        return STALLED, unsummed
    return GO, [f"{mergeable.WORKFLOW} passed on the train."]


def wait(repo: str, sha: str, deadline: float, poll: float, tick=None) -> tuple[str, list[str]]:
    """Poll the run on `sha` until [`tip_state`] settles, or `deadline` seconds pass (stalled)."""
    began, rerun = time.monotonic(), set()
    while True:
        waited = time.monotonic() - began
        runs, jobs = ci.evidence(repo, sha)
        state, lines = tip_state(runs, jobs, waited, frozenset(rerun))
        if state == RERUN:
            run = verdict.never_judged(runs, jobs)[0]
            state, lines = ci.rerun_failed(repo, run, lines)
            if state != WAIT:
                return STALLED, lines
            rerun.add(run["id"])
        elif state != WAIT:
            return state, lines
        if waited > deadline:
            for run in mergeable.unfinished(runs, jobs):  # its halves need the runners
                subprocess.run(["gh", "api", "-X", "POST", f"repos/{repo}/actions/runs/{run['id']}/cancel"],
                               capture_output=True, check=False)
            return STALLED, [f"still waiting on the train {sha[:7]} after {deadline / 60:.0f} minutes.", *lines]
        talk.say(f"train {sha[:7]}: {lines[0]}")
        if tick is not None:
            tick()
        time.sleep(poll)


def push(train: trains.Train, repo: str, root: str) -> str | None:
    """Push the train's tip to its branch; why not, or `None`."""
    name = trains.branch(train)
    delete(name, repo, root)
    for delay in (*mergeable.RETRY_DELAYS[:3], None):  # a blip must not turn trains off
        done = talk.git("push", "--quiet", "origin", f"{train.tip}:refs/heads/{name}", cwd=root)
        if done.returncode == 0:
            return None
        if delay is not None:
            time.sleep(delay)
    return f"could not push {name}: {done.stderr.strip()}"


def dispatch(repo: str, name: str) -> str | None:
    """Run CI on branch `name`; why not, or `None`. Asked again on a failure in transit."""
    for delay in (*mergeable.RETRY_DELAYS[:3], None):
        done = subprocess.run(
            ["gh", "api", "-X", "POST", f"repos/{repo}/actions/workflows/{WORKFLOW_FILE}/dispatches", "-f", f"ref={name}"],
            capture_output=True, text=True, check=False,
        )
        if done.returncode == 0:
            return None
        if delay is None or not mergeable.transport(done.stderr):
            return f"could not run CI on {name}: {done.stderr.strip()}"
        time.sleep(delay)
    return None


def delete(name: str, repo: str, root: str) -> bool:
    """Delete branch `name` on GitHub, by REST and else by git; whether it is gone. Never raises."""
    done = subprocess.run(["gh", "api", "-X", "DELETE", f"repos/{repo}/git/refs/heads/{name}"],
                          capture_output=True, text=True, check=False)
    if done.returncode == 0 or "Reference does not exist" in done.stderr:
        return True
    return talk.git("push", "--quiet", "origin", f":refs/heads/{name}", cwd=root).returncode == 0


def left_behind(listing: str) -> list[str]:
    """The `merge-train/*` branches in `git ls-remote` output."""
    names = [line.split("\t", 1)[1] for line in listing.splitlines() if "\t" in line]
    return [n.removeprefix("refs/heads/") for n in names if n.startswith(f"refs/heads/{trains.PREFIX}")]


def sweep(repo: str, root: str) -> list[str]:
    """Delete every `merge-train/*` branch a stopped watch left; the names deleted.
    Housekeeping: git not answering is no train swept, and the watch goes on."""
    listed = talk.git("ls-remote", "origin", f"refs/heads/{trains.PREFIX}*", cwd=root)
    gone = left_behind(listed.stdout) if listed.returncode == 0 else []
    return [name for name in gone if delete(name, repo, root)]
