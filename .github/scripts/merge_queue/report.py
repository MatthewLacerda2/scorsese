"""The summary an unattended run leaves behind, and its exit status."""

from __future__ import annotations

from . import mergeable
from .states import HANDED_BACK, MERGED, UNFINISHED, UNFINISHED_STATUS, UNREACHABLE


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
