"""What a pull request's description must say, read by `mergeable.py` (#1229).

The cloud brief (`.claude/skills/issue-batch/cloud-brief.md`, *Protocol*)
asks two things of every description, and both had been missed: two of the
2026-10-06 batch's nine pull requests left out `Closes #N`, so their issues
stayed open after the merge, and #1214, #1200, #1078, #1046 and #993 have no
**Gates** line, so the orchestrator could not see what had run green.

- **A branch named for an issue names it.** A head branch `N-…` is work on
  issue N, so the description either closes it (`Closes #N`, or any keyword
  GitHub honours) or says it deliberately leaves it open (`refs #N`). Neither
  is a hand-back: [`closing`] says why.
- **A Gates line says what ran.** Missing it is a warning, not a refusal —
  the run CI just made is the proof that matters to a merge — but it is said
  every time, so the habit is corrected where it is formed.

Pure functions of `gh pr view --json headRefName,body`, so the whole of it is
tested without a network. A pull request read without those fields (an older
caller) is not judged on them.
"""

from __future__ import annotations

import re

#: A head branch named for an issue: its leading number.
BRANCH = re.compile(r"^(\d+)-")

#: The keywords GitHub closes an issue on, and `refs`, the brief's way of
#: saying an issue is deliberately left open.
KEYWORDS = r"(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?|refs)"

#: A Gates line: `Gates:`, `**Gates**`, or a `Gates` heading, at a line start.
GATES = re.compile(r"^\s*(?:#+\s*)?(?:[-*]\s+)?\**Gates\b", re.MULTILINE)


def issue_of(branch: str) -> int | None:
    """The issue a head branch is named for, or `None`."""
    match = BRANCH.match(branch or "")
    return int(match.group(1)) if match else None


def names(body: str, issue: int) -> bool:
    """Whether `body` closes, or deliberately refers to, issue `issue`."""
    pattern = rf"\b{KEYWORDS}\s*:?\s+#{issue}\b"
    return re.search(pattern, body or "", re.IGNORECASE) is not None


def closing(pull: dict) -> list[str]:
    """Why the description does not name the branch's issue, or nothing.

    GitHub matches a closing keyword anywhere in a description, so a pull
    request that forgets it leaves its issue open after the merge, and nobody
    notices until the board is read again. A hand-back costs one edit of the
    description, which needs no push and no new CI run.
    """
    if "body" not in pull or "headRefName" not in pull:
        return []
    issue = issue_of(pull["headRefName"])
    if issue is None or names(pull["body"], issue):
        return []
    return [
        f"the branch is {pull['headRefName']}, but the description does not"
        f" close #{issue}.",
        f"Start it with `Closes #{issue}` (or write `refs #{issue}` if the"
        " issue is deliberately left open); editing the description needs no"
        " push and no new run.",
    ]


def gates(pull: dict) -> list[str]:
    """A warning when the description has no Gates line, or nothing."""
    if "body" not in pull or GATES.search(pull["body"] or ""):
        return []
    return [
        "Warning: the description has no **Gates** line saying what ran green"
        " on which head (cloud brief, *Protocol*)."
    ]
