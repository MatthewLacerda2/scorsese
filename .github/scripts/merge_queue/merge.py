"""Merge and verify: after a merge call failed in transit, asking whether it merged.

The merge call itself is `merge-queue.py`'s `take`. What is here is the one
thing a 502 does not answer (#495).
"""

from __future__ import annotations

import json
import subprocess
import time

from . import talk
from .states import GO, STOP, WAIT


# How long a merge call that failed in transit is given to show up as merged.
# The 502 on #490 was answered with the merge already done, but GitHub's
# pull-request view is not promised to agree the same instant. A minute is
# generous for that and short beside the ten a run costs; past it the outcome
# is *unknown*, which is handed back as unknown rather than as a refusal.
MERGE_SETTLES_SECONDS = 60


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
        talk.say(f"#{number}: {lines[0]}")
        time.sleep(min(poll, MERGE_SETTLES_SECONDS / 4))
