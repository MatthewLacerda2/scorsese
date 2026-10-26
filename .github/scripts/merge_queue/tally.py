"""The watch's aging tally, kept between watches (#1423).

A watch is bounded (`--for`), and each `watch: resume:` is a new process, so a
tally held in memory started over at every resume: on 2026-10-16 #1366 sat in
line across four watches, about five hours, never passed over three times in
any one of them. So the tally is a file beside the lock (in the common git
directory), read when a watch starts and written at every pick.

Each entry is a pull request's number and its passes. A push does not start it
over: a pull request handed back for a conflict and rebased is the broad one
the aging protects, so the only reset is `count_passes`' on taking it. A watch
on another label keeps its own file ([`path_for`]), and a `--no-merge` dry run
writes none, since its picks are not real passes. Only a watch holding the
lock writes it, so nothing races. It fails open: a missing or unreadable file
is an empty tally, which is the order without aging.
"""

from __future__ import annotations

import contextlib
import json
import os
import re
from pathlib import Path

from . import lock, watch

# The `queue` watch's file; another label's adds the label ([`path_for`]).
NAME = "merge-watch-tally.json"


def path_for(root: str, label: str = watch.QUEUE_LABEL) -> Path:
    """The tally file of the `label` watch on the repository `root` is a checkout of.

    Beside the lock, so no checkout ever shows it and every worktree shares it.
    """
    name = NAME
    if label != watch.QUEUE_LABEL:
        name = f"merge-watch-tally-{re.sub(r'[^A-Za-z0-9_-]+', '-', label)}.json"
    return lock.path_for(root).with_name(name)


def load(path: Path) -> dict[int, int]:
    """The tally last written at `path`, or an empty one."""
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        return {int(number): int(passes) for number, passes in raw.items()}
    except (OSError, ValueError, TypeError, AttributeError):
        return {}


def save(path: Path, tally: dict[int, int]) -> None:
    """Write `tally` whole, through a rename, so a reader never sees half of it.

    A write that fails is skipped, its spare file removed: the watch goes on
    with the tally in memory.
    """
    spare = path.with_name(f"{path.name}.{os.getpid()}")
    try:
        spare.write_text(
            json.dumps({str(number): passes for number, passes in sorted(tally.items())}),
            encoding="utf-8",
        )
        os.replace(spare, path)
    except OSError:
        with contextlib.suppress(OSError):
            spare.unlink(missing_ok=True)
