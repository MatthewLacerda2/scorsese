#!/usr/bin/env python3
"""`merge-queue.py N M ...` — do the waiting that serialized merging costs.

Merging stays serialized, one branch at a time, because Rust is compiled: two
pull requests can each be green alone and break `main` together. Nothing here
proposes otherwise. What this automates is **who sits through the ten minutes**
— rebase the branch, force-push it, wait for the run on the rebased head, ask
[`mergeable.judge`], merge, take the next one, repeat. A batch on 2026-08-29
paid that loop twenty-three times by hand, and the agent that paid it was
holding a worktree open the whole while.

Invoked, never a service. It runs when somebody types `make queue`, on the
pull requests they name, in the order they name them.

## Why it does not skip a run instead

#492 offered a cheaper answer first: teach `make mergeable` a fifth question —
*does this rebase need a fresh run at all?* — on the reasoning that most
rebases in that batch had an empty conflict set and a base delta touching no
crate the branch compiled.

The reasoning is not sound, and the measurement says so. A run's verdict is
about a **tree**, and it carries to another tree only when the two are the
same tree. "The base delta touched no crate the branch compiled" is not that:
if `main` gained a change to `core` and the branch changed `compositor`, the
combination has never been compiled, and a changed signature meeting a new
caller is exactly the failure serialization exists to catch. Believing
otherwise is speculative merging, which #492 puts out of scope and CLAUDE.md
rules out by name.

So the sound rule is tree identity over everything the gates read, and it was
measured against the batch that motivated the issue — the sixteen pull requests
from #468 to #489, and every CI run they produced. Of **23 re-runs, 0** tested
a tree the previous run had already tested. Four of those were pure rebases
with the branch's own files untouched, and every one of them still pulled real
Rust in from `main` — #487 rebasing across #482 moved 73 files. A rule that
fires zero times is not machinery worth having, and an unsound one that fires
often is a false green, which is the worst thing this repo can produce.

The one case where the question has a safe answer is kept, because it costs a
comparison: **a rebase that changes nothing.** When the branch is already on
`main`'s tip the head commit does not move, so there is nothing to push, no new
run to wait for, and the run already on record is a run on this exact commit —
see [`push_needed`]. That is the fifth question, in the only form that is true.

## The three things it must not do

Requirements, not caveats; each is a failure this repo has seen.

1. **It never resolves a conflict.** Empty conflict set, or it stops and hands
   the branch back naming the paths. `SYNTH_VERSION` collided twice in one
   night and both times the right answer was *the next number* — neither side —
   and taking either would leave every project on disk holding audio its own
   recipe no longer describes, under an unchanged hash. Two other branches cut
   the same seam differently and the resolution was to delete one file and keep
   the other, which no textual merge reaches. A queue allowed only the easy case
   is still worth having, because the easy case is almost all of them.
2. **It never merges on a local result alone.** It does not build anything at
   all: `make gates` is the branch author's job, run before the pull request was
   marked ready, and the cross-platform claim comes from CI because CI is a
   different computer with a different ffmpeg. The only thing consulted here is
   [`mergeable.judge`], which asks GitHub whether a run genuinely happened on
   the head commit.
3. **It never reads the mutation signal.** Mutation is a signal, it cannot fail
   a build, and it does not hold a merge. `make mutants` also fans out harder
   than this machine can carry beside anything else. Nothing below looks at it.

## What it touches, and what it leaves alone

The rebase happens in a **throwaway worktree this script creates and removes**,
detached, never in a worktree somebody is working in — an agent editing a
branch must not find it rewritten underneath. The push is
`--force-with-lease` against the head the pull request had when this started,
so a push from anywhere else in the meantime refuses rather than being
overwritten.

A merged branch's local worktree is *not* removed and the branch is *not*
deleted: that is gigabytes and a checkout somebody may still be standing in,
and the summary names them instead. Removing a worktree out from under an agent
to save disk is the same class of mistake as rebasing one.

Run it:

    python3 .github/scripts/merge-queue.py 486 488 489
    make queue PRS="486 488 489"

Python, beside `mergeable.py`, for `mergeable.py`'s own reason: it is a few
`gh` calls, a few `git` calls and a decision. It compiles nothing, reads no
cargo metadata, and imports [`mergeable.judge`] directly rather than restating
it — two definitions of "did CI pass" would drift, and the drift would be
silent. The decisions are pure functions taking plain dictionaries, so the
whole of the reasoning is tested without a network.
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "mergeable", Path(__file__).resolve().parent / "mergeable.py"
)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)

# How often GitHub is asked again. A cold CI run is about ten minutes, so a
# tighter poll buys nothing but API calls; a looser one adds its own interval
# to every branch in the queue, and the queue is the thing being shortened.
POLL_SECONDS = 30

# How long "there is no run yet" is read as *not yet* rather than as an answer.
# A force-push and the run it creates are not simultaneous — GitHub has to
# evaluate the workflow's triggers first — so reading the gap as #153 would
# refuse every branch this script pushes. After this, the silence is the
# answer, and `mergeable.no_run` says which of #153 and #429 it is.
RUN_APPEARS_SECONDS = 300

# The ceiling on one branch, in minutes. Four times a cold run, because a
# queued or re-run job can push a run well past its usual length and a queue
# that gives up early hands back a branch that was about to go green. Reaching
# it is never a merge — it is a hand-back saying the run is still out.
DEADLINE_MINUTES = 40

WAIT, GO, STOP = "wait", "go", "stop"

# What the summary calls each branch's ending. Merged and green are separate
# because `--no-merge` exists, and a queue that reported them the same would be
# claiming a merge it did not make.
MERGED, GREEN, HANDED_BACK = "merged", "green", "handed back"


def git(*args: str, cwd: str | None = None) -> subprocess.CompletedProcess:
    """`git`, captured and never fatal. Callers read `returncode` themselves."""
    return subprocess.run(
        ["git", *args], capture_output=True, text=True, check=False, cwd=cwd
    )


def ordered(numbers: list[int]) -> list[int]:
    """The queue as given, with repeats dropped and the order kept.

    A number twice is a typo and not an instruction: the second pass would find
    the pull request already merged and hand it back as a failure, which is a
    scary-looking report about nothing.
    """
    seen: set[int] = set()
    return [n for n in numbers if not (n in seen or seen.add(n))]


def conflicts(unmerged: str) -> list[str]:
    """The paths a failed rebase left conflicted, from `--diff-filter=U`.

    Named rather than counted, because the hand-back is read by whoever has to
    resolve it and "3 conflicts" sends them to go and look. It is also how the
    two known-hard cases announce themselves: a `SYNTH_VERSION` line, and two
    files that are the same seam cut twice.
    """
    return [line.strip() for line in unmerged.splitlines() if line.strip()]


def push_needed(before: str, after: str) -> bool:
    """Whether the rebase moved the head commit — and so whether to push.

    The fifth question of #492, in the only form that is airtight. If the
    rebase changed nothing, the branch was already on `main`'s tip: the tree is
    not merely equivalent to the one CI tested, it is the same commit, and the
    run on record is a run on it. Pushing anyway would rewrite the branch to
    itself and buy a second cold run for the identical tree — which is a
    literal instance of the waste this script exists to remove.
    """
    return before != after


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


def progress(
    pull: dict, runs: list[dict], jobs: dict[int, list[dict]], waited: float
) -> tuple[str, list[str]]:
    """Whether to wait, merge, or hand this branch back — and why.

    Pure, and the whole of the queue's decision. The verdict itself is
    [`mergeable.judge`]; what this adds is the distinction that a polling loop
    needs and a one-shot check does not: *not yet* against *no*.

    Order matters and mirrors `judge`'s. A failure is asked about before a run
    in flight, because a commit carrying both a red run and a live one is
    already answered — waiting out the live one would sit through ten minutes
    to be told what the red one said at the start.

    The `waited` grace on an absent run is the other half. `judge` is right to
    call no-run a refusal: it is asked once, of a commit that has been sitting
    there. Here the commit was pushed seconds ago, and a run that does not
    exist yet is the ordinary case for the first poll.
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

    if not runs:
        if waited < RUN_APPEARS_SECONDS:
            return WAIT, [
                f"no run for {short} yet; GitHub has not created one.",
                "Ordinary this soon after a push, and not yet an answer.",
            ]
        return STOP, mergeable.no_run(pull, short)

    if mergeable.failed_runs(runs):
        return STOP, mergeable.judge(pull, runs, jobs)[1]

    live = mergeable.unfinished(runs)
    if live:
        return WAIT, [
            f"a {mergeable.WORKFLOW} run for {short} is"
            f" {live[0].get('status')}."
        ]

    ok, lines = mergeable.judge(pull, runs, jobs)
    return (GO if ok else STOP), lines


def summary(results: list[tuple[int, str, str]]) -> list[str]:
    """The report, which is the only thing an unattended run leaves behind.

    One line per pull request and the reason on every one of them, including
    the merges — a queue that says "merged" and nothing else gives a reader no
    way to tell a genuine pass from a check that quietly did not happen, which
    is the confusion `make mergeable` exists to end.

    The cleanup note is here rather than done, deliberately: see the module
    doc on what this script refuses to touch.
    """
    lines = [f"#{number}: {state} — {why}" for number, state, why in results]
    merged = [n for n, state, _ in results if state == MERGED]
    if merged:
        lines.append(
            "Merged: "
            + ", ".join(f"#{n}" for n in merged)
            + ". Their worktrees and branches are still here — remove the"
            " worktrees and delete the branches when nobody is standing in one."
        )
    return lines


def say(line: str, *rest: str) -> None:
    print(f"queue: {line}", flush=True)
    for extra in rest:
        print(f"  {extra}", flush=True)


def look(number: int) -> dict:
    """The pull request, in the fields every step below reads."""
    return mergeable.gh(
        "pr",
        "view",
        str(number),
        "--json",
        "isDraft,headRefOid,headRefName,number,state,mergeable,mergeStateStatus",
    )


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


def advance(branch: str, head: str, root: str) -> tuple[str | None, list[str]]:
    """Put `branch` on `main`'s tip, remotely. Returns the new head, or why not.

    The rebase happens in a **detached, disposable worktree this function
    creates and removes**: an agent may be sitting in this branch's real
    worktree, and rewriting that underneath them is not a thing a merge queue
    gets to do.

    The push happens from inside that worktree, before it goes away, so the
    rebased commits are still referenced by something when they are sent. The
    lease is the head the pull request had when this branch's turn began, so a
    push from anywhere else in the meantime is refused rather than overwritten.

    Whether it pushed at all is [`push_needed`], which the caller asks again
    rather than being told — it is pure, and one answer is easier to trust than
    two.
    """
    with tempfile.TemporaryDirectory(prefix="scorsese-queue-") as tmp:
        work = os.path.join(tmp, "wt")
        made = git("worktree", "add", "--detach", work, head, cwd=root)
        if made.returncode != 0:
            return None, [f"could not check {branch} out: {made.stderr.strip()}"]
        try:
            done = git("rebase", "origin/main", cwd=work)
            if done.returncode != 0:
                unmerged = git(
                    "diff", "--name-only", "--diff-filter=U", cwd=work
                ).stdout
                git("rebase", "--abort", cwd=work)
                paths = conflicts(unmerged)
                return None, [
                    f"{branch} conflicts with `main`.",
                    *(
                        [f"Conflicted: {', '.join(paths)}."]
                        if paths
                        else ["The rebase stopped; git did not name a path."]
                    ),
                    "Handed back unresolved. A machine that cannot say why the"
                    " code is shaped as it is does not get to pick a side —"
                    " `SYNTH_VERSION` is the standing example, where the answer"
                    " is the next number and neither side is right.",
                ]
            fresh = git("rev-parse", "HEAD", cwd=work).stdout.strip()
            if push_needed(head, fresh):
                pushed = git(
                    "push",
                    f"--force-with-lease=refs/heads/{branch}:{head}",
                    "origin",
                    f"HEAD:refs/heads/{branch}",
                    cwd=work,
                )
                if pushed.returncode != 0:
                    return None, [
                        f"the force-push of {branch} was refused:"
                        f" {pushed.stderr.strip()}",
                        "The lease held the head this queue started from, so"
                        " something else has pushed since. Handed back.",
                    ]
            return fresh, []
        finally:
            git("worktree", "remove", "--force", work, cwd=root)


def wait_for(
    repo: str, number: int, sha: str, before: str, deadline: float, poll: float
) -> tuple[str, list[str]]:
    """Poll until the run on `sha` settles, or the deadline says stop.

    Never treats an absent check as a settled one — that is the trap the
    `ci-merge` skill names, and [`progress`] is where the two are told apart.
    [`head_state`] is the same distinction one level up: GitHub's view of the
    push lags the push.
    """
    began = time.monotonic()
    while True:
        pull = look(number)
        waited = time.monotonic() - began
        state, lines = head_state(
            pull.get("headRefOid", ""), sha, before, waited
        )
        if state == STOP:
            return state, lines
        if state == GO:
            runs, jobs = evidence(repo, sha)
            state, lines = progress(pull, runs, jobs, waited)
            if state != WAIT:
                return state, lines
        if waited > deadline:
            return STOP, [
                f"still waiting on {sha[:7]} after {deadline / 60:.0f} minutes.",
                *lines,
                "Handed back with the run still out. Nothing is red; nothing is"
                " green either.",
            ]
        say(f"#{number}: {lines[0]}")
        time.sleep(poll)


def take(repo: str, number: int, opts: argparse.Namespace) -> tuple[int, str, str]:
    """One pull request, from where it is to merged or handed back."""
    pull = look(number)
    if pull.get("state") != "OPEN":
        return number, HANDED_BACK, f"it is {str(pull.get('state')).lower()}."
    if pull.get("isDraft"):
        # Before the fetch and before the push: a draft is not a claim, so
        # there is nothing here to check and no reason to rewrite its branch.
        return number, HANDED_BACK, "it is a draft; mark it ready first."

    branch, head = pull["headRefName"], pull["headRefOid"]
    say(f"#{number} ({branch}): rebasing {head[:7]} onto origin/main.")
    git("fetch", "origin", cwd=opts.root)

    fresh, refused = advance(branch, head, opts.root)
    if fresh is None:
        say(f"#{number}: {refused[0]}", *refused[1:])
        return number, HANDED_BACK, refused[0]

    if push_needed(head, fresh):
        say(f"#{number}: pushed {fresh[:7]}; waiting for CI.")
    else:
        # The one sound skip — see `push_needed`.
        say(f"#{number}: already on `main`; the run on record is a run on it.")

    state, lines = wait_for(
        repo, number, fresh, head, opts.deadline * 60, opts.poll
    )
    say(f"#{number}: {lines[0]}", *lines[1:])
    if state != GO:
        return number, HANDED_BACK, lines[0]
    if opts.no_merge:
        return number, GREEN, lines[0]

    done = subprocess.run(
        ["gh", "pr", "merge", str(number), "--squash"],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        blocked = f"CI passed but the merge was refused: {done.stderr.strip()}"
        say(f"#{number}: {blocked}")
        return number, HANDED_BACK, blocked
    say(f"#{number}: merged.")
    return number, MERGED, lines[0]


def parse(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="merge-queue.py",
        description=(
            "Rebase, push, wait for CI and merge each pull request in turn."
            " Merging stays serialized; this only does the waiting."
        ),
    )
    parser.add_argument(
        "prs",
        metavar="PR",
        type=int,
        nargs="+",
        help="pull request numbers, merged in the order given",
    )
    parser.add_argument(
        "--no-merge",
        action="store_true",
        help="stop at green and hand each branch back rather than merging it",
    )
    parser.add_argument(
        "--deadline",
        type=float,
        default=DEADLINE_MINUTES,
        metavar="MINUTES",
        help=f"give up waiting on one branch after this long (default {DEADLINE_MINUTES})",
    )
    parser.add_argument(
        "--poll",
        type=float,
        default=POLL_SECONDS,
        metavar="SECONDS",
        help=f"how often to ask GitHub again (default {POLL_SECONDS})",
    )
    parser.add_argument(
        "--root",
        default=".",
        metavar="DIR",
        help="the git checkout to rebase in (default: the current directory)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    opts = parse(sys.argv[1:] if argv is None else argv)
    repo = mergeable.gh("repo", "view", "--json", "nameWithOwner")["nameWithOwner"]

    results = []
    for number in ordered(opts.prs):
        results.append(take(repo, number, opts))

    print()
    for line in summary(results):
        say(line)
    return 0 if all(state != HANDED_BACK for _, state, _ in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
