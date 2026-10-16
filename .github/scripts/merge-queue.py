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

A fourth, one level down: a run can conclude **failure with nothing judged**,
when GitHub found no runner for some jobs in time and cancelled them before a
single step ran (#792, run 37367713417 on #788). Its jobs that ran passed;
"failure" there is about the runners, not the code. [`never_judged`] tells it
apart from a real red — one job that genuinely failed, anywhere, keeps it red —
and the queue re-runs the failed jobs **once** and waits on the new attempt. A
second attempt that goes unjudged too ends the turn *unfinished*, like the
deadline, never handed back as red.

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
hand-back: the answer is to run the queue again — and a watch's last line,
`watch: resume: …`, is that command, exactly.

A watch also says what changed on the board — a pull request opening, turning
ready, going back to draft or closing — and what it did with each one it took,
one `watch: ` line per event, so a background `Monitor` on its output filtered
to those lines is a batch's only watch (#819, `merge-watch.py`).

## One at a time, enforced

Two queues on one repository race each other's rebases and pushes, so every
run, a list or a watch, holds an exclusive lock for its lifetime, and a second
one refuses at once with [`LOCKED_STATUS`], naming the holder's pid and how
long it has run. It is a `flock`, which the kernel drops with the process, so a
crash leaves nothing stale (#1346, `merge_queue/lock.py`).

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

## Where each part lives

This file is the orchestration — one pull request's turn ([`take`]), a list
and a watch over turns, and `main`. The rest is the `merge_queue` package
beside it, one responsibility a module (#1136): `rebase` (rebase and push, or
Dependabot's own), `ci` (waiting on the run) over `verdict` (the pure
decision), `merge` (verifying a merge that failed in transit), `report` (the
summary and exit status), `numbering` (#729's collisions), `lock` (one queue
at a time per repository, #1346), `talk` (git,
GitHub's view of the pull request, and the queue's output), `options` (the
command line) and `states` (the words they share). Each calls another's
functions through the module (`ci.wait_for(…)`), never by imported name, so a
test replaces a function where it is defined and every caller sees it.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

# The package beside this script, importable however the script was loaded —
# `python3` puts this directory on the path, a test's `importlib` does not.
HERE = str(Path(__file__).resolve().parent)
if HERE not in sys.path:
    sys.path.insert(0, HERE)

from merge_queue import ci, lock, merge, mergeable, options, rebase, report, talk, verdict, watch  # noqa: E402,F401
from merge_queue.numbering import migration_number, numbering, schema_version  # noqa: E402,F401
from merge_queue.options import DEADLINE_MINUTES, POLL_SECONDS, parse, since  # noqa: E402,F401
from merge_queue.states import (  # noqa: E402,F401
    GO, GREEN, HANDED_BACK, LATE, LOCKED_STATUS, MERGED, NOT_REACHED, RERUN, STOP,
    UNFINISHED, UNFINISHED_STATUS, UNJUDGED, UNREACHABLE, WAIT,
)
from merge_queue.verdict import RUN_APPEARS_SECONDS, head_state, progress  # noqa: E402,F401
from merge_queue.ci import wait_for  # noqa: E402,F401
from merge_queue.merge import MERGE_SETTLES_SECONDS, landed, settled  # noqa: E402,F401
from merge_queue.rebase import BOT_REBASE, bot_head, conflicts, push_needed  # noqa: E402,F401
from merge_queue.report import status, summary  # noqa: E402,F401


def ordered(numbers: list[int]) -> list[int]:
    """The queue as given, with repeats dropped and the order kept.

    A number twice is a typo and not an instruction: the second pass would find
    the pull request already merged and hand it back as a failure, which is a
    scary-looking report about nothing.
    """
    seen: set[int] = set()
    return [n for n in numbers if not (n in seen or seen.add(n))]


def take(
    repo: str,
    number: int,
    opts: argparse.Namespace,
    heads: dict[int, str] | None = None,
    tick=None,
) -> tuple[int, str, str]:
    """One pull request, from where it is to merged or handed back.

    `tick` is [`wait_for`]'s, passed through.

    `heads`, when given, is told the last head this call knew the pull request
    by — the one it found, or the one it pushed. The watch keeps it so that a
    branch handed back is not taken again until somebody pushes a new head.
    """
    pull = talk.look(number)
    if heads is not None:
        heads[number] = pull.get("headRefOid", "")
    if pull.get("state") != "OPEN":
        return number, HANDED_BACK, f"it is {str(pull.get('state')).lower()}."
    if pull.get("isDraft"):
        # Before the fetch and before the push: a draft is not a claim, so
        # there is nothing here to check and no reason to rewrite its branch.
        return number, HANDED_BACK, "it is a draft; mark it ready first."
    # Before the push too: rebasing and waiting out CI for a branch that may not
    # close what it closes, or re-blesses a reference it does not explain,
    # would be ten minutes spent on a refusal (#1221, #1217).
    refused = mergeable.closing.refusal(
        mergeable.closing.closed_by(mergeable.gh, repo, pull)
    ) or mergeable.reblessed.refusal(
        mergeable.reblessed.evidence(mergeable.gh, repo, pull), pull.get("body") or ""
    )
    if refused:
        talk.say(f"#{number}: {refused[0]}", *refused[1:])
        return number, HANDED_BACK, refused[0]

    branch, head = pull["headRefName"], pull["headRefOid"]
    talk.git("fetch", "origin", cwd=opts.root)
    # One `--deadline` covers the whole turn, Dependabot's rebase included, so
    # a bot pull request cannot take two and break the watch's budget.
    began = time.monotonic()
    if watch.is_bot(pull):
        talk.say(f"#{number} ({branch}): Dependabot's branch; it rebases itself.")
        fresh, refused = rebase.bot_head(number, head, opts.root, opts)
    else:
        talk.say(f"#{number} ({branch}): rebasing {head[:7]} onto origin/main.")
        fresh, refused = rebase.advance(branch, head, opts.root)
    if fresh is None:
        talk.say(f"#{number}: {refused[0]}", *refused[1:])
        return number, HANDED_BACK, refused[0]

    if heads is not None:
        heads[number] = fresh
    if rebase.push_needed(head, fresh):
        talk.say(f"#{number}: now at {fresh[:7]}; waiting for CI.")
    else:
        # The one sound skip — see `push_needed`.
        talk.say(f"#{number}: already on `main`; the run on record is a run on it.")

    left = opts.deadline * 60 - (time.monotonic() - began)
    state, lines = ci.wait_for(repo, number, fresh, head, left, opts.poll, tick)
    talk.say(f"#{number}: {lines[0]}", *lines[1:])
    if state in (LATE, UNJUDGED):
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
            talk.say(f"#{number}: {blocked}")
            return number, HANDED_BACK, blocked
        talk.say(f"#{number}: the merge call failed in transit: {failure}")
        state, after = merge.confirm(number, opts.poll)
        talk.say(f"#{number}: {after[0]}", *after[1:])
        if state != GO:
            return number, HANDED_BACK, after[0]
    talk.say(f"#{number}: merged.")
    return number, MERGED, lines[0]


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
            talk.say(f"#{number}: {outage}")
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

    Every result is also said as it happens, as one [`watch.outcome`] line,
    and the board's changes as [`watch.transitions`] lines between polls.
    """
    results: list[tuple[int, str, str]] = []
    dropped: dict[int, str] = {}
    issues: dict[int, list[str]] = {}
    merges: dict[frozenset[str], bool] = {}
    behind_main: dict[frozenset[str], list[str] | None] = {}
    passes: dict[int, int] = {}
    board = watch.Board(opts.since)
    stop_taking = time.monotonic() + opts.watch_for * 60

    def tick() -> None:
        for line in board.poll():
            talk.tell(line)

    def record(number: int, state: str, why: str) -> None:
        results.append((number, state, why))
        if state == MERGED:
            board.landed.add(number)
        talk.tell(watch.outcome(number, state, why))

    talk.say(
        f"watching for ready pull requests labelled `{opts.label}`, for"
        f" {opts.watch_for:g} minutes."
    )
    try:
        while True:
            tick()
            pulls = watch.waiting(opts.label)
            if time.monotonic() >= stop_taking:
                for number in watch.in_line(pulls, dropped):
                    record(number, UNFINISHED, "still in line when the watch ended.")
                return results
            labels = watch.issue_labels(pulls, issues)
            paths = watch.against_main(watch.contenders(pulls, dropped), opts.root, behind_main)
            for number, why in watch.behind(pulls, dropped, paths):
                # Its turn could only end in this hand-back; have it now (#1345).
                talk.say(f"#{number}: {why}")
                record(number, HANDED_BACK, why)
                dropped[number] = next(
                    p.get("headRefOid", "") for p in pulls if p["number"] == number
                )
            clashes = watch.clashing(
                watch.contenders(pulls, dropped), opts.root, merges
            )
            pull = watch.pick(pulls, dropped, labels, clashes, passes)
            if pull is None:
                time.sleep(watch.IDLE_SECONDS)
                continue
            note = watch.reordered(pulls, dropped, labels, clashes, passes)
            if note:
                talk.say(note)
            number = pull["number"]
            watch.count_passes(passes, pulls, dropped, number)
            heads: dict[int, str] = {}
            try:
                outcome = take(repo, number, opts, heads, tick)
            except mergeable.Unreachable as outage:
                talk.say(f"#{number}: {outage}")
                record(number, UNREACHABLE, str(outage))
                return results
            record(*outcome)
            # Merged too: the listing can still show a merged pull request for
            # a poll or two, and taking it again would report it handed back.
            dropped[number] = heads.get(number, pull.get("headRefOid", ""))
    except mergeable.Unreachable as outage:
        # Asking what is cleared, rather than working on one: no number to name.
        talk.say(str(outage))
        record(0, UNREACHABLE, str(outage))
        return results


def main(argv: list[str] | None = None) -> int:
    opts = options.parse(sys.argv[1:] if argv is None else argv)
    try:
        held = lock.acquire(lock.path_for(opts.root))  # noqa: F841 — kept until exit
    except lock.Held as busy:
        talk.say(str(busy), "Nothing was attempted.")
        return LOCKED_STATUS
    try:
        repo = mergeable.gh("repo", "view", "--json", "nameWithOwner")
    except mergeable.Unreachable as outage:
        talk.say(str(outage), "Nothing was attempted. Run it again once GitHub answers.")
        return mergeable.UNREACHABLE_STATUS
    take_all = run_watch if opts.watch else run_queue
    results = take_all(repo["nameWithOwner"], opts)

    print()
    for line in report.summary(results):
        talk.say(line)
    if opts.watch:
        talk.tell(watch.resume(opts))
    return report.status(results)


if __name__ == "__main__":
    raise SystemExit(main())
