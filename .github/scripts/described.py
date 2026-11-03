#!/usr/bin/env python3
"""`described.py N` — would the merge path take pull request N's description?

The checks `make mergeable` and `make queue` apply to a pull request's
description and comments, in one place, so a coder can ask them before
readying and the queue asks the very same function (#1701):

- what it closes may be closed ([`closing`]): no `planning` or `human` issue,
  no keyword aimed at a pull request or a closed issue;
- every modified reference is named under `Re-blessed` with a reason
  ([`reblessed`]);
- a risky path has an `Independent review` comment at its level, its heading
  on the comment's first line ([`reviewed`]);
- a branch named `N-…` closes or refers to #N, and a missing **Gates** line is
  warned about ([`protocol`]). The queue asks these two only once CI has
  passed, so a red run is still what its refusal names; here they come last.

Every one is a description edit or a comment away from passing, and needs no
push and no run, so it costs nothing to ask after every edit, draft or not.

    python3 .github/scripts/described.py 1702

It reads GitHub over REST only (`gh api`, shaped by `pulls.py`), as the queue
and `make mergeable` do, so all three judge alike in a cloud container.

Pure but for [`gather`]'s `gh`, [`github`] and `main`.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import closing  # noqa: E402
import protocol  # noqa: E402
import pulls  # noqa: E402
import reblessed  # noqa: E402
import reviewed  # noqa: E402


def gather(gh, repo: str, pull: dict) -> dict:
    """`pull` with what the checks ask GitHub for: `closes`, `reblessed`,
    `reviewed`. Every call is REST."""
    pull["closes"] = closing.closed_by(gh, repo, pull)
    pull["reblessed"] = reblessed.evidence(gh, repo, pull)
    files = (pull["reblessed"] or {}).get("files") or []
    pull["reviewed"] = reviewed.evidence(gh, repo, pull, files)
    return pull


def refusal(pull: dict) -> list[str]:
    """What the merge path refuses before it reads any run; empty when nothing.

    Each part is read only when [`gather`] (or a caller) put it there, so a
    caller that asked GitHub for less is not judged on what it did not ask.
    """
    return (
        closing.refusal(pull.get("closes") or {})
        or reblessed.refusal(pull.get("reblessed") or {}, pull.get("body") or "")
        or reviewed.refusal(pull.get("reviewed") or {}, pull)
    )


def passing(pull: dict, lines: list[str]) -> tuple[bool, list[str]]:
    """A passing verdict, unless the description breaks the protocol: a branch
    named for an issue that does not close it is refused, a missing Gates line
    is a warning (`protocol.py`)."""
    refused = protocol.closing(pull)
    if refused:
        return False, refused
    return True, lines + protocol.gates(pull)


def check(pull: dict) -> tuple[bool, list[str]]:
    """Every description check, in the order the merge path meets them."""
    refused = refusal(pull)
    if refused:
        return False, refused
    return passing(pull, ["the description says what the merge path asks."])


def github():
    """`mergeable.gh`: its retries in transit, its exit on an answer."""
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "mergeable", Path(__file__).resolve().parent / "mergeable.py")
    mergeable = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mergeable)
    return mergeable.gh, mergeable.Unreachable, mergeable.UNREACHABLE_STATUS


def main(argv: list[str]) -> int:
    if len(argv) != 1 or not argv[0].isdigit():
        print("usage: described.py PULL_REQUEST_NUMBER", file=sys.stderr)
        return 2
    number = argv[0]
    gh, unreachable, status = github()
    try:
        repo = pulls.repository()
        pull = gather(gh, repo, pulls.view(gh, repo, number))
    except unreachable as outage:
        print(f"described: #{number}: {outage}", file=sys.stderr)
        print("  Nothing was decided. Ask again once GitHub answers.", file=sys.stderr)
        return status
    ok, lines = check(pull)
    head, *rest = lines
    where = sys.stdout if ok else sys.stderr
    print(f"described: #{number}: {head}", file=where)
    for line in rest:
        print(f"  {line}", file=where)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
