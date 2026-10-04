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
pull requests they name, in the order they name them — or, with `--watch`, on
the ones labelled for it, for a bounded time, and then it exits.

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
   The same goes for the two numbers that collide *without* a conflict — a
   migration number or a `SCHEMA_VERSION` bump `main` already took — which
   [`numbering`] finds after the rebase and hands back unpushed (#729).
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

**Dependabot's branch is never pushed to.** Dependabot stops maintaining a
branch somebody else has pushed to, so a force-push from here would orphan the
very pull request being merged. Dependabot rebases its own branch instead (and
cancels its own runs as it does): a bot pull request that is not on `main`'s
tip is asked to, with a [`BOT_REBASE`] comment, and the queue waits for a head
that is ([`bot_head`]), then judges that head like any other. Ported from
rusty's queue, where it has merged Dependabot's pull requests since 2026-09-30
(#721).

A merged branch's local worktree is *not* removed and the branch is *not*
deleted: that is gigabytes and a checkout somebody may still be standing in,
and the summary names them instead. Removing a worktree out from under an agent
to save disk is the same class of mistake as rebasing one.

## A hand-back skips the entry; it does not stop the queue

Decided in #495, and stated here once. A queue can be a *train* — each car
built on the one before, so a derailment strands everything behind it — or a
list of **independent entries**. This one is the second, by construction:
every branch is rebased onto `main` as `main` is *at its own turn*, and CI
judges that tree. Whatever happened to the entry before — merged, red,
conflicted, or a merge whose outcome is still unknown — the next one is tested
against the real `main` and not against a guess about it. So nothing a
hand-back leaves behind can make a later merge unsound, and stopping would only
turn one branch's problem into every branch's wait, in exactly the unattended
run the queue exists for. Each hand-back is named in the summary, and the exit
status is non-zero if there was any.

## GitHub's answer about a state is not always the state

Three times now, each met in use. The runs listing is eventually consistent
([`progress`]'s grace); a force-push lags its own poll ([`head_state`]'s); and
the merge call can answer **502 Bad Gateway having already merged** (#495,
merging #490). So a failed merge is sorted by what failed. A **transport**
failure — a 5xx, a timeout, a reset connection — says nothing about the merge,
and the pull request is asked whether it merged before anything is concluded
([`mergeable.transport`], [`landed`]). A **refusal** GitHub reasoned about — a
409, "not mergeable", a protected branch — is a real no, and is handed back with
no second call.

Every *question* the queue asks goes through `mergeable.gh`, which retries a
failure in transit for a few minutes before calling GitHub unreachable. An
unreachable GitHub stops the queue with its own status and its own line in the
summary, never as a hand-back: nothing was decided, and the answer is to run
the same queue again (#615).

## Or let it watch

`--watch` drops the list: the queue asks GitHub every poll for the ready pull
requests carrying the `queue` label, takes them in `CLAUDE.md`'s label
priority and then by age, and stops taking new ones after `--for` minutes so
it fits the harness's two-hour cap. Every guarantee above holds per pull
request, because each one taken goes through the same [`take`]. Which one, why
a label, and how a hand-back is kept from being retried every poll are
`merge-watch.py`'s module doc (#690).

A branch whose run is still out at its deadline, and a pull request still in
line when a watch ends, are **unfinished** — neither red nor green, and not a
hand-back: the answer is to run the queue again.

Run it:

    python3 .github/scripts/merge-queue.py 486 488 489
    make queue PRS="486 488 489"
    make queue WATCH=1

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
import json
import os
import re
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

_spec = importlib.util.spec_from_file_location(
    "merge_watch", Path(__file__).resolve().parent / "merge-watch.py"
)
watch = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(watch)
# One `mergeable` between the two, so the `Unreachable` the watch's questions
# raise is the class this script catches.
watch.mergeable = mergeable

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
# it is never a merge and never a verdict: the branch is *unfinished*.
DEADLINE_MINUTES = 40

# How long a merge call that failed in transit is given to show up as merged.
# The 502 on #490 was answered with the merge already done, but GitHub's
# pull-request view is not promised to agree the same instant. A minute is
# generous for that and short beside the ten a run costs; past it the outcome
# is *unknown*, which is handed back as unknown rather than as a refusal.
MERGE_SETTLES_SECONDS = 60

# The command Dependabot obeys to rebase its own branch onto `main`.
BOT_REBASE = "@dependabot rebase"

WAIT, GO, STOP, LATE = "wait", "go", "stop", "late"

# What the summary calls each branch's ending. Merged and green are separate
# because `--no-merge` exists, and a queue that reported them the same would be
# claiming a merge it did not make. Unreachable is separate from handed back
# because it is not a verdict: GitHub stopped answering, nothing was decided,
# and the right response is to run the same queue again rather than to go and
# fix the branch (#615). Unfinished is separate from handed back for the same
# reason: a run still out at the deadline, or a pull request still in line when
# a watch ends, has had nothing decided about it (#690).
MERGED, GREEN, HANDED_BACK = "merged", "green", "handed back"
UNREACHABLE, NOT_REACHED, UNFINISHED = "unreachable", "not reached", "unfinished"

# The exit status when nothing was handed back and nothing was unreachable,
# but something is unfinished: run the queue again. Not 1, which says read a
# hand-back; not 3, which says GitHub stopped answering.
UNFINISHED_STATUS = 4


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


# The two numbers a clean rebase can still collide on (#729). Both are chosen
# by counting up from `main`, so two branches cut from the same `main` pick the
# same one, and git sees no conflict: different migration filenames, or the
# identical `SCHEMA_VERSION` edit on both sides. CI finds either a full round
# later; a filename listing and a `git show` find it before the push.
MIGRATIONS = "crates/server/migrations"
SCHEMA_FILE = "crates/core/src/project.rs"
SCHEMA_LINE = re.compile(r"pub const SCHEMA_VERSION: u32 = (\d+);")


def migration_number(path: str) -> int | None:
    """The `0017` of `…/0017_library_midi.sql`, or None for anything else."""
    name = path.rsplit("/", 1)[-1]
    head = name.split("_", 1)[0]
    return int(head) if name.endswith(".sql") and head.isdigit() else None


def schema_version(source: str | None) -> int | None:
    """`SCHEMA_VERSION` as `project.rs` declares it, or None if it does not."""
    found = SCHEMA_LINE.search(source or "")
    return int(found.group(1)) if found else None


def numbering(
    on_main: list[str],
    added: list[str],
    versions: tuple[int | None, int | None, int | None, int | None],
) -> list[str]:
    """Why this rebased branch has taken a number `main` already holds, if it has.

    `on_main` is the migration paths on `origin/main`; `added` the ones the
    rebased branch adds. `versions` is `SCHEMA_VERSION` at the branch's fork
    point, its old head, the rebased head and `origin/main`. Pure, so the
    whole judgement is tested without git, like [`push_needed`]. The queue
    only names the number; renumbering is the author's, because the migration
    step and the fixtures move with it.

    A Markdown-only branch or a Dependabot bump adds no migration and leaves
    `SCHEMA_VERSION` alone, so it passes here without being told to skip.
    """
    taken = {migration_number(p) for p in on_main} - {None}
    mine = sorted(
        (n, p) for p in added if (n := migration_number(p)) is not None
    )
    nums = [n for n, _ in mine]
    clash = [p for n, p in mine if n in taken or nums.count(n) > 1]
    lines = []
    if clash:
        lines.append(
            f"Migration number already taken: {', '.join(clash)}; renumber"
            f" from {max(taken | {0}) + 1}."
        )
    fork, head, rebased, main = versions
    if head != fork and rebased is not None and rebased == main:
        lines.append(
            f"`SCHEMA_VERSION` {rebased} is already `main`'s: a sibling's bump"
            f" took it. Renumber to {rebased + 1}, with its migration step and"
            " fixtures."
        )
    return lines


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

    if mergeable.failed_runs(runs):
        return STOP, mergeable.judge(pull, runs, jobs)[1]

    live = mergeable.unfinished(runs)
    if live:
        return WAIT, [
            f"a {mergeable.WORKFLOW} run for {short} is"
            f" {live[0].get('status')}."
        ]

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


def landed(pull: dict) -> bool:
    """Whether GitHub's pull-request record says it merged.

    `state` and `mergedAt` both, because either alone is the answer. The
    commit-ancestry check the issue also offered is deliberately absent: this
    queue merges with `--squash`, so the branch head is **never** an ancestor
    of `main` — the squash is a new commit — and that check would answer no to
    every merge that happened.
    """
    return pull.get("state") == "MERGED" or bool(pull.get("mergedAt"))


def settled(pull: dict | None, waited: float) -> tuple[str, list[str]]:
    """After a merge call failed in transit: merged, not yet known, or unknown.

    `pull` is `None` when asking GitHub failed too, which is likely in the
    same outage that produced the 5xx and is no more an answer than it was.
    Only a record that says merged is taken as merged; silence is waited on
    for [`MERGE_SETTLES_SECONDS`], then handed back as *unknown* — a hand-back
    that says "refused" about a merge that happened invites somebody to merge
    it again.
    """
    if pull is not None and landed(pull):
        return GO, ["the merge call failed in transit, but it merged."]
    if waited < MERGE_SETTLES_SECONDS:
        return WAIT, ["the merge call failed in transit; asking whether it merged."]
    return STOP, [
        "the merge call failed in transit and GitHub has not said it merged.",
        "Whether it did is unknown: look at the pull request before merging it"
        " again.",
    ]


def summary(results: list[tuple[int, str, str]]) -> list[str]:
    """The report, which is the only thing an unattended run leaves behind.

    One line per pull request and the reason on every one of them, including
    the merges — a queue that says "merged" and nothing else gives a reader no
    way to tell a genuine pass from a check that quietly did not happen, which
    is the confusion `make mergeable` exists to end.

    The cleanup note is here rather than done, deliberately: see the module
    doc on what this script refuses to touch.
    """
    lines = [
        f"{f'#{number}' if number else 'watch'}: {state} — {why}"
        for number, state, why in results
    ]
    merged = [n for n, state, _ in results if state == MERGED]
    if merged:
        lines.append(
            "Merged: "
            + ", ".join(f"#{n}" for n in merged)
            + ". Their worktrees and branches are still here — remove the"
            " worktrees and delete the branches when nobody is standing in one."
        )
    if any(state == UNREACHABLE for _, state, _ in results):
        lines.append(
            "GitHub stopped answering, so the queue stopped: nothing past that"
            " point was decided. Run it again with the same list — a pull"
            " request already merged is skipped as merged, and a head already"
            " pushed is not pushed again."
        )
    if any(state == UNFINISHED for _, state, _ in results):
        lines.append(
            "Unfinished is not red: nothing was decided about those. Run the"
            " queue again to take them."
        )
    return lines


def status(results: list[tuple[int, str, str]]) -> int:
    """The exit status: the most urgent thing a reader has to do about the run."""
    states = {state for _, state, _ in results}
    if UNREACHABLE in states:
        return mergeable.UNREACHABLE_STATUS
    if HANDED_BACK in states:
        return 1
    return UNFINISHED_STATUS if UNFINISHED in states else 0


def run_queue(repo: str, opts: argparse.Namespace) -> list[tuple[int, str, str]]:
    """Every pull request in turn, stopping at the first GitHub cannot answer.

    Stopping rather than skipping ahead, because the order was given on
    purpose and an outage long enough to exhaust `mergeable.gh`'s retries is
    not going to spare the next branch. What was not reached is said so, one
    line each, rather than left out of a summary that reads as the whole run.
    """
    results = []
    numbers = ordered(opts.prs)
    for at, number in enumerate(numbers):
        try:
            results.append(take(repo, number, opts))
        except mergeable.Unreachable as outage:
            say(f"#{number}: {outage}")
            results.append((number, UNREACHABLE, str(outage)))
            results.extend(
                (later, NOT_REACHED, "the queue stopped before it.")
                for later in numbers[at + 1 :]
            )
            break
    return results


def run_watch(repo: str, opts: argparse.Namespace) -> list[tuple[int, str, str]]:
    """Take cleared pull requests as they appear, until `--for` runs out.

    One at a time, through the same [`take`] as a list — merging stays
    serialized and the watch only decides what comes next ([`watch.pick`]).
    A branch taken once is not taken again on the same head, whatever the
    outcome: a merge is done, a hand-back waits for a push, a green under `--no-merge` has
    nothing more to do, and an unfinished one belongs to the next watch.

    GitHub going quiet stops the watch exactly as it stops a list, with the
    reason in the summary.
    """
    results: list[tuple[int, str, str]] = []
    dropped: dict[int, str] = {}
    issues: dict[int, list[str]] = {}
    merges: dict[frozenset[str], bool] = {}
    stop_taking = time.monotonic() + opts.watch_for * 60
    say(
        f"watching for ready pull requests labelled `{opts.label}`, for"
        f" {opts.watch_for:g} minutes."
    )
    try:
        while True:
            pulls = watch.waiting(opts.label)
            if time.monotonic() >= stop_taking:
                results.extend(
                    (number, UNFINISHED, "still in line when the watch ended.")
                    for number in watch.in_line(pulls, dropped)
                )
                return results
            labels = watch.issue_labels(pulls, issues)
            clashes = watch.clashing(
                watch.contenders(pulls, dropped), opts.root, merges
            )
            pull = watch.pick(pulls, dropped, labels, clashes)
            if pull is None:
                time.sleep(watch.IDLE_SECONDS)
                continue
            note = watch.reordered(pulls, dropped, labels, clashes)
            if note:
                say(note)
            number = pull["number"]
            heads: dict[int, str] = {}
            try:
                outcome = take(repo, number, opts, heads)
            except mergeable.Unreachable as outage:
                say(f"#{number}: {outage}")
                results.append((number, UNREACHABLE, str(outage)))
                return results
            results.append(outcome)
            # Merged too: the listing can still show a merged pull request for
            # a poll or two, and taking it again would report it handed back.
            dropped[number] = heads.get(number, pull.get("headRefOid", ""))
    except mergeable.Unreachable as outage:
        # Asking what is cleared, rather than working on one: no number to name.
        say(str(outage))
        results.append((0, UNREACHABLE, str(outage)))
        return results


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
        "isDraft,headRefOid,headRefName,number,state,mergeable,mergeStateStatus,author",
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


def merge_record(number: int) -> dict | None:
    """The pull request's merge fields, or `None` if GitHub did not answer.

    Not [`look`]: that goes through `mergeable.gh`, which spends minutes
    retrying and then raises, and this is asked during the very outage that
    made it necessary — [`confirm`] is already the retry loop, with its own
    window.
    """
    done = subprocess.run(
        ["gh", "pr", "view", str(number), "--json", "state,mergedAt"],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        return None
    try:
        return json.loads(done.stdout)
    except json.JSONDecodeError:
        return None


def confirm(number: int, poll: float) -> tuple[str, list[str]]:
    """Ask, until [`settled`] says something other than *wait*."""
    began = time.monotonic()
    while True:
        state, lines = settled(merge_record(number), time.monotonic() - began)
        if state != WAIT:
            return state, lines
        say(f"#{number}: {lines[0]}")
        time.sleep(min(poll, MERGE_SETTLES_SECONDS / 4))


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
            clashes = collisions(head, work)
            if clashes:
                return None, [
                    f"{branch} rebases cleanly but collides on a number.",
                    *clashes,
                    "Handed back unpushed: CI would only find it ten minutes on.",
                ]
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


def on_tip(sha: str, root: str) -> bool:
    """Whether `sha` already contains `origin/main`'s tip (fetched by the caller).

    The sha is fetched by itself first: a head Dependabot has just pushed is not
    on any ref this checkout has fetched yet.
    """
    git("fetch", "--quiet", "origin", sha, cwd=root)
    return git("merge-base", "--is-ancestor", "origin/main", sha, cwd=root).returncode == 0


def bot_head(
    number: int, head: str, root: str, opts: argparse.Namespace
) -> tuple[str | None, list[str]]:
    """Dependabot's own rebase, waited for: a head on `main`'s tip, or why not.

    Never force-pushed from here (see the module doc). Asked once with
    [`BOT_REBASE`], then polled until a new head on `main`'s tip appears or
    `--deadline` runs out — the second is a hand-back, remembered by head like
    any other, so the watch does not ask again until Dependabot pushes.
    """
    if on_tip(head, root):
        return head, []
    # Not `mergeable.gh`: if the comment fails, the wait below still answers.
    subprocess.run(
        ["gh", "pr", "comment", str(number), "--body", BOT_REBASE],
        capture_output=True,
        check=False,
    )
    began = time.monotonic()
    while time.monotonic() - began < opts.deadline * 60:
        time.sleep(opts.poll)
        seen = look(number).get("headRefOid", "")
        if seen != head and on_tip(seen, root):
            return seen, []
        say(f"#{number}: waiting for Dependabot to rebase {head[:7]}.")
    return None, [
        f"Dependabot did not rebase {head[:7]} onto `main` within"
        f" {opts.deadline:.0f} minutes."
    ]


def collisions(head: str, work: str) -> list[str]:
    """[`numbering`] over the rebased tree in `work`; `head` is the old head."""

    def listing(*args: str) -> list[str]:
        # The trailing slash matters: without it `ls-tree` names the directory
        # itself rather than the files in it.
        return git(*args, "--", f"{MIGRATIONS}/", cwd=work).stdout.split()

    def version(rev: str) -> int | None:
        shown = git("show", f"{rev}:{SCHEMA_FILE}", cwd=work)
        return schema_version(shown.stdout if shown.returncode == 0 else None)

    fork = git("merge-base", head, "origin/main", cwd=work).stdout.strip()
    return numbering(
        listing("ls-tree", "--name-only", "origin/main"),
        listing("diff", "--name-only", "--diff-filter=A", "origin/main", "HEAD"),
        (version(fork), version(head), version("HEAD"), version("origin/main")),
    )


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
            return LATE, [
                f"still waiting on {sha[:7]} after {deadline / 60:.0f} minutes.",
                *lines,
                "Unfinished, with the run still out. Nothing is red; nothing is"
                " green either. Run the queue again to pick it back up.",
            ]
        say(f"#{number}: {lines[0]}")
        time.sleep(poll)


def take(
    repo: str,
    number: int,
    opts: argparse.Namespace,
    heads: dict[int, str] | None = None,
) -> tuple[int, str, str]:
    """One pull request, from where it is to merged or handed back.

    `heads`, when given, is told the last head this call knew the pull request
    by — the one it found, or the one it pushed. The watch keeps it so that a
    branch handed back is not taken again until somebody pushes a new head.
    """
    pull = look(number)
    if heads is not None:
        heads[number] = pull.get("headRefOid", "")
    if pull.get("state") != "OPEN":
        return number, HANDED_BACK, f"it is {str(pull.get('state')).lower()}."
    if pull.get("isDraft"):
        # Before the fetch and before the push: a draft is not a claim, so
        # there is nothing here to check and no reason to rewrite its branch.
        return number, HANDED_BACK, "it is a draft; mark it ready first."

    branch, head = pull["headRefName"], pull["headRefOid"]
    git("fetch", "origin", cwd=opts.root)
    # One `--deadline` covers the whole turn, Dependabot's rebase included, so
    # a bot pull request cannot take two and break the watch's budget.
    began = time.monotonic()
    if watch.is_bot(pull):
        say(f"#{number} ({branch}): Dependabot's branch; it rebases itself.")
        fresh, refused = bot_head(number, head, opts.root, opts)
    else:
        say(f"#{number} ({branch}): rebasing {head[:7]} onto origin/main.")
        fresh, refused = advance(branch, head, opts.root)
    if fresh is None:
        say(f"#{number}: {refused[0]}", *refused[1:])
        return number, HANDED_BACK, refused[0]

    if heads is not None:
        heads[number] = fresh
    if push_needed(head, fresh):
        say(f"#{number}: now at {fresh[:7]}; waiting for CI.")
    else:
        # The one sound skip — see `push_needed`.
        say(f"#{number}: already on `main`; the run on record is a run on it.")

    left = opts.deadline * 60 - (time.monotonic() - began)
    state, lines = wait_for(repo, number, fresh, head, left, opts.poll)
    say(f"#{number}: {lines[0]}", *lines[1:])
    if state == LATE:
        return number, UNFINISHED, lines[0]
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
        failure = done.stderr.strip()
        if not mergeable.transport(failure):
            blocked = f"CI passed but the merge was refused: {failure}"
            say(f"#{number}: {blocked}")
            return number, HANDED_BACK, blocked
        say(f"#{number}: the merge call failed in transit: {failure}")
        state, after = confirm(number, opts.poll)
        say(f"#{number}: {after[0]}", *after[1:])
        if state != GO:
            return number, HANDED_BACK, after[0]
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
        nargs="*",
        help="pull request numbers, merged in the order given (or use --watch)",
    )
    parser.add_argument(
        "--watch",
        action="store_true",
        help=(
            f"take ready pull requests labelled `{watch.QUEUE_LABEL}` as they"
            " appear, fewest conflicts first, then label priority, then age,"
            " instead of a list"
        ),
    )
    parser.add_argument(
        "--for",
        dest="watch_for",
        type=float,
        default=watch.FOR_MINUTES,
        metavar="MINUTES",
        help=(
            f"with --watch: stop taking new pull requests after this long"
            f" (default {watch.FOR_MINUTES}; with --deadline at most"
            f" {watch.CAP_MINUTES})"
        ),
    )
    parser.add_argument(
        "--label",
        default=watch.QUEUE_LABEL,
        metavar="NAME",
        help=f"with --watch: the go-ahead label to look for (default {watch.QUEUE_LABEL})",
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
    opts = parser.parse_args(argv)
    if opts.watch and opts.prs:
        parser.error("--watch takes no pull request numbers; it finds them.")
    if not opts.watch and not opts.prs:
        parser.error("which pull requests? Name them, or pass --watch.")
    if opts.watch:
        problem = watch.budget_error(opts.watch_for, opts.deadline)
        if problem:
            parser.error(problem)
    return opts


def main(argv: list[str] | None = None) -> int:
    opts = parse(sys.argv[1:] if argv is None else argv)
    try:
        repo = mergeable.gh("repo", "view", "--json", "nameWithOwner")
    except mergeable.Unreachable as outage:
        say(str(outage), "Nothing was attempted. Run it again once GitHub answers.")
        return mergeable.UNREACHABLE_STATUS
    take_all = run_watch if opts.watch else run_queue
    results = take_all(repo["nameWithOwner"], opts)

    print()
    for line in summary(results):
        say(line)
    return status(results)


if __name__ == "__main__":
    raise SystemExit(main())
