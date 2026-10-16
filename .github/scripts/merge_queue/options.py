"""`merge-queue.py`'s command line."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone

from . import watch


# How often GitHub is asked again. A cold CI run is about ten minutes, so a
# tighter poll buys nothing but API calls; a looser one adds its own interval
# to every branch in the queue, and the queue is the thing being shortened.
POLL_SECONDS = 30


# The ceiling on one branch, in minutes. Four times a cold run, because a
# queued or re-run job can push a run well past its usual length and a queue
# that gives up early hands back a branch that was about to go green. Reaching
# it is never a merge and never a verdict: the branch is *unfinished*.
DEADLINE_MINUTES = 40


def since(text: str) -> datetime:
    """`--since`'s value: an ISO 8601 time, read as UTC when it names no zone."""
    try:
        when = datetime.fromisoformat(text)
    except ValueError as bad:
        raise argparse.ArgumentTypeError(
            f"{text!r} is not an ISO 8601 time, e.g. 2026-10-06T16:00:00Z"
        ) from bad
    return when if when.tzinfo else when.replace(tzinfo=timezone.utc)


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
            " appear, fewest conflicts first (none passed over more than"
            f" {watch.AGED_PASSES} times), then label priority, then age,"
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
        "--since",
        type=since,
        default=None,
        metavar="TIME",
        help=(
            "with --watch: report board changes only for pull requests opened"
            " at or after this ISO 8601 time (default: every open one)"
        ),
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
