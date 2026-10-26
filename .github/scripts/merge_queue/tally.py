"""The watch's aging tally, kept between watches (#1423).

A watch is bounded (`--for`), and each `watch: resume:` is a new process, so a
tally held in memory started over at every resume: on 2026-10-16 #1366 sat in
line across four watches, about five hours, never passed over three times in
any one of them. So the tally is a file beside the lock ([`NAME`], in the
common git directory), read when a watch starts and written at every pick.

Each entry is a pull request's number, the head it was counted against and its
passes; a head that moved starts over ([`carried`]). Only a
watch holding the lock writes it, so nothing races. It fails open: a missing
or unreadable file is an empty tally, which is the order without aging.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

from . import lock

# Beside the lock, so no checkout ever shows it and every worktree shares it.
NAME = "merge-watch-tally.json"


def path_for(root: str) -> Path:
    """The tally file for the repository `root` is a checkout of."""
    return lock.path_for(root).with_name(NAME)


def carried(
    saved: dict[int, tuple[str, int]], pulls: list[dict]
) -> dict[int, int]:
    """The passes of `saved` still standing: those on a head still listed.

    A pull request that moved head or left the listing starts over (#1423).
    """
    heads = {pull.get("number"): pull.get("headRefOid") for pull in pulls}
    return {
        number: passes
        for number, (head, passes) in saved.items()
        if heads.get(number) == head
    }


def stamped(passed: dict[int, int], pulls: list[dict]) -> dict[int, tuple[str, int]]:
    """The watch's `count_passes` tally with each head, as [`carried`] reads it back.

    Only the pull requests still listed are kept, so the file never grows.
    """
    heads = {pull.get("number"): pull.get("headRefOid", "") for pull in pulls}
    return {
        number: (heads[number], passes)
        for number, passes in passed.items()
        if number in heads
    }


def load(path: Path) -> dict[int, tuple[str, int]]:
    """The tally last written at `path`, or an empty one."""
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        return {int(number): (str(head), int(passes)) for number, (head, passes) in raw.items()}
    except (OSError, ValueError, TypeError, AttributeError):
        return {}


def save(path: Path, tally: dict[int, tuple[str, int]]) -> None:
    """Write `tally` whole, through a rename, so a reader never sees half of it.

    A write that fails is skipped: the watch goes on with the tally in memory.
    """
    spare = path.with_name(f"{NAME}.{os.getpid()}")
    try:
        spare.write_text(
            json.dumps({str(number): list(entry) for number, entry in sorted(tally.items())}),
            encoding="utf-8",
        )
        os.replace(spare, path)
    except OSError:
        pass
