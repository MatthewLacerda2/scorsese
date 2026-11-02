#!/usr/bin/env python3
"""`regression_local.py [N]` — CI's `regression test` job, from this checkout (#1701).

`make gates` cannot run that job: it builds the base. So a coder used to learn
only after readying that a reproducing test named API the fix adds, or sat
where no target names it. This runs `regression.py`'s `plan` and `run` here,
over the same functions, in a scratch worktree `on_base` removes afterwards:

    make regression PR=1702          # what the description owes, on its base
    make regression BASE=<commit>    # the changed tests on that commit, nothing read

It builds the base once (its own crates; dependencies are already built).
Pure but for [`local`].
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path
from typing import Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

import closing  # noqa: E402
import regression  # noqa: E402
from described import github, repository  # noqa: E402

Unreachable = github()[1]

def asked(number: str, gh: Callable | None, head: Path, base: str):
    """Pull request `number`, the `bug` issues it closes, and its base: `BASE`
    or the merge-base with `origin/<base>`. `None` for the pull request when
    this checkout's `HEAD` is not its head, which would judge other code."""
    if gh is None:
        gh, _, _ = github()
    repo = repository()
    pull = gh("api", f"repos/{repo}/pulls/{number}")
    sha = (pull.get("head") or {}).get("sha")
    here = regression.git("rev-parse", "HEAD", cwd=head).strip()
    if sha and sha != here:
        print(f"regression: #{number}'s head is {sha[:12]}, this checkout is at {here[:12]}:"
              " check out its branch and push what is committed first.", file=sys.stderr)
        return None, [], base
    fixed = regression.bugs(closing.closed_by(gh, repo, {"body": pull.get("body") or ""}))
    if not base:
        ref = (pull.get("base") or {}).get("ref", "main")
        regression.git("fetch", "--quiet", "origin", ref, cwd=head)
        base = regression.git("merge-base", f"origin/{ref}", "HEAD", cwd=head).strip()
        tip = regression.git("rev-parse", f"origin/{ref}", cwd=head).strip()
        if base != tip:
            print(f"note: the branch is behind origin/{ref}; CI judges its tip, so"
                  " rebase first for CI's answer.")
    return pull, fixed, base


USAGE = "usage: regression_local.py [PULL_REQUEST_NUMBER], with BASE=<commit> or a number"


def local(argv: list[str], gh: Callable | None = None) -> int:
    """The job's `plan` and `run`, from this checkout.

    With a pull request number, its description decides what is owed, as
    `plan` does, and its base is where this branch left `origin/<base>` (the
    merge-base, fetched first): the base CI uses once the branch is rebased
    onto it, which a ready branch is. `BASE=<commit>` names one instead; given
    alone, nothing is read from GitHub and the changed tests are run on it
    whatever they fix. Exit 0 when a changed test fails on the base or nothing
    is owed, 1 when the job would refuse, 2 when it cannot ask.

    The test files are copied from the working tree, so it refuses one with
    changes: what it proves is the commit."""
    head, base = Path.cwd(), os.environ.get("BASE", "")
    if len(argv) > 1 or (argv and not argv[0].isdigit()) or not (argv or base):
        print(USAGE, file=sys.stderr)
        return 2
    if regression.git("status", "--porcelain", "--untracked-files=no", cwd=head).strip():
        print("regression: the working tree has uncommitted changes; commit or stash"
              " them first: the run proves the commit, not the tree.", file=sys.stderr)
        return 2
    fixed: list[int] = []
    body = ""
    if argv:
        try:
            pull, fixed, base = asked(argv[0], gh, head, base)
        except (Unreachable, subprocess.CalledProcessError) as failed:
            print(f"regression: could not ask: {failed}. Nothing was decided.", file=sys.stderr)
            return 2
        if pull is None:
            return 2
        body = pull.get("body") or ""
    found = regression.targets(regression.changed(head, base), lambda p: (head / p).exists())
    if argv:
        refused = regression.owed(fixed, body, found)
        if refused:
            print("\n".join(["Refused:"] + refused))
            return 1
        if not fixed or regression.excused(body):
            print(f"excused: {regression.excused(body)}" if fixed
                  else "it closes no `bug` issue: nothing is owed.")
            return 0
    elif not found:
        print("\n".join(["Refused:", "no changed test names a target.", regression.HOW]))
        return 1
    print(f"base {base[:12]}; runs on it, building it once:", *map(str, found), sep="\n  ")
    env = {**os.environ, "REGRESSION_HEAD": str(head)}
    refused = regression.verdict(fixed, regression.on_base(head, base, found, env))
    print("\n".join(["Refused:"] + refused) if refused else "a changed test fails on the base.")
    return 1 if refused else 0

if __name__ == "__main__":
    sys.exit(local(sys.argv[1:]))
