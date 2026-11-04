"""Everything the queue asks of git and GitHub's pull request, and everything it says.

The two questions every step asks — `git`, and the pull request as GitHub has
it now — and the two ways it speaks: a `queue:` line of narration, and an event
line the watch prints as it is.

Every other module calls these as `talk.look(…)` and never imports the name,
so a test replaces one in exactly one place, here.
"""

from __future__ import annotations

import subprocess

from . import mergeable


def git(*args: str, cwd: str | None = None) -> subprocess.CompletedProcess:
    """`git`, captured and never fatal. Callers read `returncode` themselves."""
    return subprocess.run(
        ["git", *args], capture_output=True, text=True, check=False, cwd=cwd
    )


def look(repo: str, number: int) -> dict:
    """The pull request, in the fields every step below reads (`pulls.from_rest`)."""
    return mergeable.pulls.view(mergeable.gh, repo, number)


def say(line: str, *rest: str) -> None:
    print(f"queue: {line}", flush=True)
    for extra in rest:
        print(f"  {extra}", flush=True)


def tell(line: str, *rest: str) -> None:
    """An event line, already carrying [`watch.EVENT`]: printed as it is, any
    further lines indented beneath it, as [`say`] does."""
    print(line, flush=True)
    for extra in rest:
        print(f"  {extra}", flush=True)
