"""What a pull request closes, and whether the merge path may close it (#1221).

`CLAUDE.md` *Overrides*: an issue labelled `planning` is never started while
the label is on it — the one rule the user cannot override — and *Stage
labels*: `planning` and `human` both mean *do not start*. A pull request that
closes such an issue started it. Until this, nothing checked that; prose did.

So `make mergeable` and `make queue` both refuse one, naming the issue and the
label. The label is read as it stands **at merge time**: the user may remove
it and greenlight the work, and a pull request queued after that merges.

The same pass refuses a closing keyword that points at a pull request or at an
issue already closed: a typo'd number closes nothing GitHub will tell anybody
about, and the issue that was meant stays open after the merge.

Which issues a pull request closes is read two ways and joined, because each
misses something: GitHub's `closingIssuesReferences` holds the issues linked
in the sidebar too, and never a pull request or a number it could not resolve;
the body is where a keyword pointing at the wrong thing is still visible.

Pure but for [`closed_by`], which asks GitHub over REST (`issues/N` answers for
a pull request number too, and says so with a `pull_request` key).
"""

from __future__ import annotations

import re

# The labels that mean *do not start* (`CLAUDE.md`, *Stage labels*).
STAGES = ("planning", "human")

# GitHub's closing keywords, then `#N` in this repository. GitHub matches them
# anywhere in a description, quoted or negated alike (`CLAUDE.md`, *Flow*), so
# this does too: a PR meaning to leave an issue open writes `refs #N`.
KEYWORD = re.compile(
    r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\b:?\s+#(\d+)\b", re.IGNORECASE
)


def referenced(pull: dict) -> list[int]:
    """Every issue number this pull request closes, by keyword or by link."""
    numbers = {int(found) for found in KEYWORD.findall(pull.get("body") or "")}
    numbers |= {
        ref["number"]
        for ref in pull.get("closingIssuesReferences") or []
        if ref.get("number")
    }
    return sorted(numbers)


def labels(issue: dict) -> list[str]:
    """An issue's label names, from REST (`name` objects) or plain strings."""
    return [
        label.get("name", "") if isinstance(label, dict) else str(label)
        for label in issue.get("labels") or []
    ]


def refusal(closes: dict[int, dict]) -> list[str]:
    """Why this pull request may not close what it closes; empty when it may.

    `closes` maps each number from [`referenced`] to GitHub's REST answer for
    it. The first line is the verdict, as every refusal on the merge path.
    """
    wrong, staged = [], []
    for number, issue in sorted(closes.items()):
        if "pull_request" in issue:
            wrong.append(f"it closes #{number}, which is a pull request, not an issue.")
        elif issue.get("state") == "closed":
            wrong.append(f"it closes #{number}, which is already closed.")
        staged += [
            f"it closes #{number}, which is labelled `{stage}`."
            for stage in STAGES
            if stage in labels(issue)
        ]
    lines = staged + wrong
    if staged:
        lines.append(
            "`planning` and `human` mean do not start (CLAUDE.md, *Stage"
            " labels*; *Overrides*: never with `planning` on). Only the user"
            " removes the label; queue it again once they have."
        )
    if wrong:
        lines.append("Fix the number, or write `refs #N` for an issue left open.")
    return lines


def both_stages(issues: list[dict]) -> list[str]:
    """The open issues carrying both stage labels: at most one is allowed.

    Not a merge hold — which label stays is the user's call — so it is listed
    in the batch report (`make checks`) for them to settle.
    """
    return [
        f"#{issue['number']} {issue.get('title', '')}".rstrip()
        for issue in issues
        if "pull_request" not in issue
        and all(stage in labels(issue) for stage in STAGES)
    ]


def closed_by(gh, repo: str, pull: dict) -> dict[int, dict]:
    """GitHub's answer for every number [`referenced`] finds, through `gh`."""
    return {number: gh("api", f"repos/{repo}/issues/{number}") for number in referenced(pull)}
