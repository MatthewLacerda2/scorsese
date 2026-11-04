"""Who rides together ([`form`]), and who goes alone ([`alone`]) (#1757).

The line is walked in the watch's own order (`merge-watch.py`'s `ordered`: the
overdue first, fewest conflicts, label priority, age), each candidate put on
the train so far ([`train.couple`]). One that conflicts with it, or takes a
number it holds, is passed over for this round and keeps its place, so the
passed-over count still counts it. The first in line sets the round: if it
cannot ride, it is taken alone, the queue's one-at-a-time turn, and so is a
train that found nobody to ride with it: a group of one is that turn.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

from . import train as trains


# The most pull requests one train carries. Rusty measured it on 2026-11-04
# over 140 pull-request runs, 7 red (p ≈ 0.05): with a red train halved, the
# expected CI rounds per pull request are 0.60 at 2, 0.44 at 4 and 0.40 at 8,
# so past four the gain is a few hundredths while a red train sends twice as
# many back through the halves.
CAP = 4

# What a member may not touch. The merge path and CI (the scripts that give
# `CI ok` its verdict included), because a train would
# prove the queue or a workflow by running the other one; the lockfiles,
# because two members resolving dependencies apart is a merge git calls clean
# and cargo `--locked` refuses (`issue-batch`: one lockfile owner per wave).
ALONE_PREFIXES = (".github/scripts/merge_queue/", ".github/workflows/")
ALONE_FILES = (
    "Makefile", "Cargo.lock", "app/Cargo.lock", "web/bun.lock",
    ".github/scripts/merge-queue.py", ".github/scripts/merge-watch.py",
    ".github/scripts/mergeable.py", ".github/scripts/pulls.py",
    ".github/scripts/described.py", ".github/scripts/ci-paths.py",
    ".github/scripts/ci-ok.py", ".github/scripts/regression.py",
)

_spec = importlib.util.spec_from_file_location("ci_paths", Path(__file__).resolve().parent.parent / "ci-paths.py")
ci_paths = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ci_paths)


def alone(files: list[str]) -> str | None:
    """Why a change to `files` takes its turn alone, or `None`.

    A change CI's `changes` job calls Markdown-only (`ci-paths.py`, the same
    list) goes alone too: alone its run is about a minute with every build
    skipped, where a train costs it a full round, and a broken member could
    drag it through the halves.
    """
    if files and not ci_paths.runs_for(ci_paths.PATTERNS, files):
        return "it is Markdown-only, which merges in a minute alone"
    hit = next((f for f in files if f in ALONE_FILES or f.startswith(ALONE_PREFIXES)), None)
    return f"it changes `{hit}`, which goes alone" if hit else None


def form(
    line: list[dict], train: trains.Train, git, refuse, cap: int = CAP
) -> tuple[list[dict], trains.Train | None, list[str]]:
    """This round's members, the train they ride (`None`: the first goes alone),
    and one `watch:` note per pull request passed over.

    `line` is the ready pull requests in order; `train` is empty, on the `main`
    to land on; `refuse(pull)` says why a pull request may not ride ([`alone`],
    its own run not green, its description refused, red in a train before), or
    `None`.
    """
    first, notes, members = line[0], [], []
    why = refuse(first)
    if why:
        return [first], None, [f"watch: #{first['number']} goes alone: {why}."]
    for pull in line:
        if len(members) == cap:
            break
        number = pull["number"]
        why = refuse(pull) if members else None
        if why:
            notes.append(f"watch: #{number} waits for a train of its own: {why}.")
            continue
        car, why = trains.couple(train, number, pull.get("headRefOid", ""), git)
        if car is None:
            if not members:
                return [first], None, [f"watch: #{number} goes alone: {why}."]
            riding = ", ".join(f"#{n}" for n in train.numbers)
            notes.append(f"watch: #{number} waits for the next train: {why} ({riding}).")
            continue
        train.cars.append(car)
        members.append(pull)
    if len(members) == 1:
        return [first], None, notes
    return members, train, notes
