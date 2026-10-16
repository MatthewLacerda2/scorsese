"""One queue at a time per repository: the lock every run takes (#1346).

On 2026-10-15 a session restarted while its background watch kept running as
an orphan, and the resumed session started another. For about fifty minutes
both polled the same pull requests, raced each other's rebases and pushes, and
wrote the same log. GitHub's own checks refused the loser each time, so nothing
was lost, but nothing here noticed either.

So every run — a watch or a one-off `make queue PRS=…` — takes an exclusive
`flock` on [`NAME`] in the repository's **common** git directory, which every
worktree of a clone shares, and holds it until the process ends. The kernel
releases it when the process dies however it dies, so a crash cannot leave a
stale lock behind, and there is nothing to clean up. The holder writes its pid
and start time into the file, so a second run refuses with a sentence naming
who holds it and for how long ([`Held`]).
"""

from __future__ import annotations

import fcntl
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import IO

from . import talk

# The lock file, beside git's own state, so no checkout ever shows it.
NAME = "merge-queue.lock"


class Held(Exception):
    """Another queue holds the lock; the message says who, and what to do."""


def path_for(root: str) -> Path:
    """The lock file for the repository `root` is a checkout of.

    The common git directory, not the worktree's own, so a queue started from
    any worktree of the clone sees the others' lock.
    """
    found = talk.git("rev-parse", "--git-common-dir", cwd=root)
    common = found.stdout.strip() if found.returncode == 0 else ".git"
    return (Path(root) / common).resolve() / NAME


def holder(text: str, now: datetime) -> str:
    """Who the lock file says holds it, in words: pid and how long."""
    pid, _, started = text.strip().partition(" ")
    if not pid:
        return "a queue whose pid is not written yet"
    try:
        minutes = (now - datetime.fromisoformat(started)).total_seconds() / 60
    except ValueError:
        return f"pid {pid}"
    return f"pid {pid}, running for {minutes:.0f} minutes (since {started})"


def acquire(path: Path, now: datetime | None = None) -> IO[str]:
    """Take the lock and keep it for as long as the returned file stays open.

    Raises [`Held`] at once, never waits, when another process holds it.
    """
    now = now or datetime.now(timezone.utc)
    handle = open(path, "a+", encoding="utf-8")  # noqa: SIM115 — held for the run
    try:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        handle.seek(0)
        who = holder(handle.read(), now)
        handle.close()
        raise Held(
            f"another queue is already running on this repository: {who}. Two"
            " would race each other's rebases and pushes (#1346). Let it"
            " finish, or stop it and run this again."
        ) from None
    handle.seek(0)
    handle.truncate()
    handle.write(f"{os.getpid()} {now.isoformat(timespec='seconds')}\n")
    handle.flush()
    return handle
