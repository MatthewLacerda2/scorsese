"""Merge and verify: the squash call ([`squash`]), and after one failed in
transit, asking whether it merged (#495).

A turn alone (`merge-queue.py`'s `take`) and a train's landing (`landing`)
both merge through [`squash`], so the two sort a failure alike.
"""

from __future__ import annotations

import json
import subprocess
import time

from . import mergeable, talk
from .states import GO, HANDED_BACK, MERGED, STOP, WAIT


# How long a merge call that failed in transit is given to show up as merged.
# The 502 on #490 was answered with the merge already done, but GitHub's
# pull-request view is not promised to agree the same instant. A minute is
# generous for that and short beside the ten a run costs; past it the outcome
# is *unknown*, which is handed back as unknown rather than as a refusal.
MERGE_SETTLES_SECONDS = 60

# What [`squash`] answers besides merged and handed back. `UNKNOWN`: it failed
# in transit and never showed up merged. `BASE_MOVED`: GitHub kept answering
# that `main` moved under the call, which a merge made seconds before it does
# (rusty's #1785 review) and which is no fault of the pull request's.
UNKNOWN, BASE_MOVED = "unknown", "base moved"

# How often, and how far apart, a "Base branch was modified" is asked again.
BASE_RETRIES, BASE_GAP_SECONDS = 3, 5.0
BASE_MODIFIED = "Base branch was modified"


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


def merge_record(repo: str, number: int) -> dict | None:
    """The pull request's merge fields, or `None` if GitHub did not answer.

    Not [`look`]: that goes through `mergeable.gh`, which spends minutes
    retrying and then raises, and this is asked during the very outage that
    made it necessary — [`confirm`] is already the retry loop, with its own
    window.
    """
    done = subprocess.run(
        ["gh", "api", f"repos/{repo}/pulls/{number}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        return None
    try:
        return mergeable.pulls.from_rest(json.loads(done.stdout))
    except (json.JSONDecodeError, AttributeError):
        return None


def confirm(repo: str, number: int, poll: float) -> tuple[str, list[str]]:
    """Ask, until [`settled`] says something other than *wait*."""
    began = time.monotonic()
    while True:
        state, lines = settled(merge_record(repo, number), time.monotonic() - began)
        if state != WAIT:
            return state, lines
        talk.say(f"#{number}: {lines[0]}")
        time.sleep(min(poll, MERGE_SETTLES_SECONDS / 4))


def squash(repo: str, number: int, sha: str, poll: float) -> tuple[str, str]:
    """Squash-merge pull request `number`, pinned to `sha`: GitHub refuses a head
    that moved since. [`MERGED`], [`HANDED_BACK`] (a refusal GitHub reasoned
    about), [`UNKNOWN`] or [`BASE_MOVED`], with one line why.

    A failure in transit is never repeated, only asked about ([`confirm`]):
    repeating a merge that may already have happened is how one gets made twice.
    """
    for attempt in range(BASE_RETRIES + 1):
        done = subprocess.run(
            mergeable.pulls.merge_call(repo, number, sha),
            capture_output=True,
            text=True,
            check=False,
        )
        failure = done.stderr.strip()
        if done.returncode == 0:
            return MERGED, "merged."
        if BASE_MODIFIED not in failure:
            break
        if attempt == BASE_RETRIES:
            return BASE_MOVED, f"GitHub kept answering that `main` moved: {failure}"
        talk.say(f"#{number}: {failure}; asking again in {BASE_GAP_SECONDS:g}s.")
        time.sleep(BASE_GAP_SECONDS)
    if not mergeable.transport(failure):
        blocked = f"CI passed but the merge was refused: {failure}"
        talk.say(f"#{number}: {blocked}")
        return HANDED_BACK, blocked
    talk.say(f"#{number}: the merge call failed in transit: {failure}")
    state, after = confirm(repo, number, poll)
    talk.say(f"#{number}: {after[0]}", *after[1:])
    return (MERGED, after[0]) if state == GO else (UNKNOWN, after[0])
