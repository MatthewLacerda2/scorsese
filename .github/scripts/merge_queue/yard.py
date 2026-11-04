"""The watch's merge trains (#1757): up to [`group.CAP`] pull requests tested on one tree, landed together.

Ported from MatthewLacerda2/rusty's merge train (`merging/train/`, its #1785),
which settled the design the day before; adapted to this queue's modules.

## What keeps it sound

**What merges is exactly the tree CI ran.** The train is built as GitHub will
land it: a chain of single-parent commits on `origin/main`, one per member,
each the tree `git merge-tree` writes for that head on the chain so far
([`train.couple`]). CI is dispatched on the chain's tip, pushed to a
`merge-train/*` branch ([`rails`]); a green run lands the members, each
squash-merged in order and pinned to its own head ([`merge.squash`]); then
`main`'s tree is compared with the train's ([`landing.check`]). A difference
stops the watch ([`STOPPED`], exit 6). If `main` moved while the train ran, it
is rebuilt on the new `main` and retested, never landed. This is `merge-queue.py`'s
rule, *a run's verdict carries only to the same tree*, kept for a group.

**Nothing is pushed to a member's head**, so a member must already merge
cleanly onto `main` and the chain so far. One that conflicts, or takes a
number the chain holds, waits for the next train; the first in line that
cannot ride takes the one-at-a-time turn, which rebases, pushes and waits.

**Every per-pull-request judgement stays one** ([`Yard.refuse`]): a member's
own head has a green run that `mergeable.judge` passes, with the description's
checks (`described.py`: what it closes, `Re-blessed`, `Independent review`, the
protocol) and the `regression` job a `bug` owes, which its own run holds.
The description is read again just before landing ([`landing.moved`]).

## Red, and the halves

A red train is split in half and each half is tested on its own, on top of
the latest `main`, before any new train ([`Yard.waiting`]); a green half lands,
and a half of one takes the one-at-a-time turn, which hands it back red with
its run named. A pull request that was in a red train goes alone until its
head moves. A run that never judged the train (no runner, the deadline) halves
it too, marking nobody red.

## When the watch runs trains

Only under `--watch`, and only while `main`'s ruleset lets a member that does
not hold `main`'s tip merge ([`rails.landable`]); otherwise one at a time, as
before, said in one line. A watch killed mid-train leaves a `merge-train/*`
branch behind; the next watch deletes every such branch before its first take.
"""

from __future__ import annotations

import argparse
import functools

from . import ci, group, landing, merge, mergeable, rails, talk, train as trains, watch
from .states import GO, GREEN, MERGED, REQUEUED, STOPPED


class Yard:
    """The trains of one watch. `fx` carries the effects ([`effects`]): `start()`,
    `git(*args)`, `files(number)`, `judged(pull)` (why its own head or description
    refuses it, or `None`), `refusal(pull)`, `push(train)`, `dispatch(branch)`,
    `wait(sha, tick)`, `delete(branch)`, `look(number)`, `squash(number, sha)`,
    `main_state(base)` (`main`'s tree and first-parent count since `base`, or
    `None`), `label`, `no_merge` and `say`."""

    def __init__(self, fx):
        self.fx = fx
        self.waiting: list[list[tuple[int, str]]] = []  # halves, tested before any new train
        self.red: dict[int, str] = {}  # the head each pull request had in a red train
        self.off: str | None = None  # why trains stopped for this watch

    def refuse(self, pull: dict) -> str | None:
        """Why `pull` may not ride a new train, or `None`. Cheapest question first."""
        number, head = pull["number"], pull.get("headRefOid", "")
        if watch.is_bot(pull):
            return "Dependabot rebases its own branch"
        if self.red.get(number) == head:
            return "this head was in a red train"
        files = self.fx.files(number)
        if files is None:
            return "GitHub did not list its files"
        return group.alone(files) or self.fx.judged(pull)

    def form(self, line: list[dict]) -> tuple[list[dict], trains.Train | None, list[str]]:
        """This round's members and their train (`None`: the first goes alone), and the notes."""
        if self.off:
            return [line[0]], None, []
        train = self.fx.start()
        if train is None:
            return [line[0]], None, ["watch: git could not read `origin/main`; taking the first alone."]
        by = {p["number"]: p for p in line}
        while self.waiting:
            half = [by[n] for n, head in self.waiting.pop(0) if n in by and by[n].get("headRefOid") == head]
            if half:
                return group.form(half, train, self.fx.git, self.fx.judged, cap=len(half))
        return group.form(line, train, self.fx.git, self.refuse)

    def ride(self, train: trains.Train, tick=None) -> list[tuple[int, str, str]]:
        """Test one train, then land it or split it: every member's result."""
        fx, listed = self.fx, ", ".join(f"#{n}" for n in train.numbers)
        state, lines = self.test(train, tick)
        fx.say(f"watch: train {listed}: {lines[0]}", *lines[1:])
        if self.off:
            return [(n, REQUEUED, f"trains are off for this watch: {self.off}") for n in train.numbers]
        if state != GO:
            if state == rails.RED:
                self.red.update((car.number, car.head) for car in train.cars)
            self.waiting[:0] = trains.halves(train)
            return [(n, REQUEUED, f"its train was {state}; its half is tested next.") for n in train.numbers]
        if fx.no_merge:
            return [(n, GREEN, f"its train {train.tip[:7]} is green.") for n in train.numbers]
        return self.land(train)

    def test(self, train: trains.Train, tick) -> tuple[str, list[str]]:
        """The push, the dispatch and the wait; the branch is deleted after.
        A push or dispatch refused turns trains off for the watch: it would be
        refused again, and the one-at-a-time turn needs neither."""
        name = trains.branch(train)
        refused = self.fx.push(train)
        try:
            refused = refused or self.fx.dispatch(name)
            if refused:
                self.off = refused
                return rails.STALLED, [refused, "Merging one at a time for the rest of this watch."]
            self.fx.say(f"watch: testing {', '.join(f'#{n}' for n in train.numbers)} together on {name} ({train.tip[:7]}).")
            return self.fx.wait(train.tip, tick)
        finally:
            self.fx.delete(name)

    def land(self, train: trains.Train) -> list[tuple[int, str, str]]:
        """A green train: checked for staleness and departures, landed, and `main`'s tree checked."""
        fx, cars = self.fx, [(car.number, car.head) for car in train.cars]
        now = fx.main_state(train.base)
        if now is None or now[1] != 0:
            # `main` moved while the train ran (or git could not say): what landing
            # now would leave is untested. Not red: rebuilt first, on the new `main`.
            self.waiting.insert(0, cars)
            fx.say(f"watch: `main` moved past {train.base[:7]} while the train ran; retaking it on the new `main`.")
            return [(n, REQUEUED, "its train went stale; retaken on the new `main`.") for n in train.numbers]
        gone = dict(landing.departed(train, fx.look, self.moved))
        if gone:
            stay = [(n, head) for n, head in cars if n not in gone]
            if stay:
                self.waiting.insert(0, stay)
            return [(n, REQUEUED, f"not landed: {gone[n]}." if n in gone else "a member left its train; retaken without it.")
                    for n in train.numbers]
        results, landed, unknown = landing.land(train, fx.look, self.moved, fx.squash)
        if landed or unknown:
            now = fx.main_state(train.base)
            halt = landing.check(train, landed, unknown, *now) if now else (
                f"{landing.MISMATCH}: git could not read `main` after the landing to compare it with"
                f" the train's {train.tree_after(landed)[:12]}; compare them by hand.")
            if halt:
                fx.say(f"watch: STOPPED: {halt}")
                results.append((0, STOPPED, halt))
        return results

    def moved(self, pull: dict, car: trains.Car) -> str | None:
        return landing.moved(pull, car, self.fx.label, self.fx.refusal)


def judged(repo: str, pull: dict) -> str | None:
    """Why `pull`'s own head may not ride, or `None`: `mergeable.judge` over its
    own runs, with what the description's checks ask GitHub for."""
    try:
        gathered = mergeable.described.gather(mergeable.gh, repo, dict(pull))
        runs, jobs = ci.evidence(repo, pull.get("headRefOid", ""))
    except (mergeable.Unreachable, SystemExit) as down:
        return f"GitHub did not answer about it ({down})"
    ok, lines = mergeable.judge(gathered, runs, jobs)
    return None if ok else f"its own head is not cleared: {lines[0].rstrip('.')}"


def refusal(repo: str, pull: dict) -> list[str]:
    """The description's checks, asked now (`described.py`)."""
    return mergeable.described.refusal(mergeable.described.gather(mergeable.gh, repo, dict(pull)))


def files(repo: str, number: int) -> list[str] | None:
    """The paths pull request `number` changes, `None` when GitHub failed."""
    found, page = [], 1
    try:
        while True:
            batch = mergeable.gh("api", f"repos/{repo}/pulls/{number}/files?per_page=100&page={page}")
            found += [f.get("filename", "") for f in batch]
            if len(batch) < 100:
                return found
            page += 1
    except (mergeable.Unreachable, SystemExit):
        return None


def main_state(git, root: str, base: str) -> tuple[str, int] | None:
    """`origin/main`'s tree, fetched now, and its first-parent commits since
    `base`; `None` when either is unknown. A failed fetch is unknown, never the
    old `origin/main`."""
    if talk.git("fetch", "--quiet", "origin", "main", cwd=root).returncode != 0:
        return None
    status, tree = git("rev-parse", "--verify", "--quiet", "origin/main^{tree}")
    counted, count = git("rev-list", "--first-parent", "--count", f"{base}..origin/main")
    if status != 0 or not tree.strip() or counted != 0 or not count.strip().isdigit():
        return None
    return tree.strip(), int(count.strip())


def effects(repo: str, opts: argparse.Namespace) -> argparse.Namespace:
    """The `fx` a [`Yard`] runs on, bound to this repository and checkout."""
    git = functools.partial(watch._git, opts.root)

    def start() -> trains.Train | None:
        fetched = talk.git("fetch", "--quiet", "origin", "main", cwd=opts.root).returncode == 0
        return trains.start(git) if fetched else None

    return argparse.Namespace(
        start=start,
        git=git,
        files=functools.partial(files, repo),
        judged=functools.partial(judged, repo),
        refusal=functools.partial(refusal, repo),
        push=lambda train: rails.push(train, repo, opts.root),
        dispatch=functools.partial(rails.dispatch, repo),
        wait=lambda sha, tick: rails.wait(repo, sha, opts.deadline * 60, opts.poll, tick),
        delete=lambda name: rails.delete(name, repo, opts.root),
        look=functools.partial(talk.look, repo),
        squash=lambda number, sha: merge.squash(repo, number, sha, opts.poll),
        main_state=functools.partial(main_state, git, opts.root),
        label=opts.label,
        no_merge=opts.no_merge,
        say=talk.tell,
    )


def open_yard(repo: str, opts: argparse.Namespace) -> Yard | None:
    """A watch's yard, or `None` (one at a time) when `main`'s rules refuse a train, said."""
    why = rails.landable(repo)
    if why:
        talk.tell(f"{watch.EVENT} merging one at a time, not in trains of {group.CAP}: {why}.")
        return None
    for name in rails.sweep(repo, opts.root):
        talk.say(f"deleted {name}, a merge train a stopped watch left behind.")
    talk.tell(f"{watch.EVENT} merging in trains of up to {group.CAP}, one CI run a train.")
    return Yard(effects(repo, opts))
