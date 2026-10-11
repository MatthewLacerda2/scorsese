"""Putting a branch on `main`'s tip: rebase and push, or Dependabot's own rebase.

The queue's first step for every pull request, and the one with the most it
must not do: see `merge-queue.py`'s module doc on conflicts, worktrees, the
lease, and Dependabot's branch.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import tempfile
import time

from . import numbering, talk
from .numbering import MIGRATIONS, SCHEMA_FILE


# The command Dependabot obeys to rebase its own branch onto `main`.
BOT_REBASE = "@dependabot rebase"


def conflicts(unmerged: str) -> list[str]:
    """The paths a failed rebase left conflicted, from `--diff-filter=U`.

    Named rather than counted, because the hand-back is read by whoever has to
    resolve it and "3 conflicts" sends them to go and look. It is also how the
    two known-hard cases announce themselves: a `SYNTH_VERSION` line, and two
    files that are the same seam cut twice.
    """
    return [line.strip() for line in unmerged.splitlines() if line.strip()]


def push_needed(before: str, after: str) -> bool:
    """Whether the rebase moved the head commit — and so whether to push.

    The fifth question of #492, in the only form that is airtight. If the
    rebase changed nothing, the branch was already on `main`'s tip: the tree is
    not merely equivalent to the one CI tested, it is the same commit, and the
    run on record is a run on it. Pushing anyway would rewrite the branch to
    itself and buy a second cold run for the identical tree — which is a
    literal instance of the waste this script exists to remove.
    """
    return before != after


def advance(branch: str, head: str, root: str) -> tuple[str | None, list[str]]:
    """Put `branch` on `main`'s tip, remotely. Returns the new head, or why not.

    The rebase happens in a **detached, disposable worktree this function
    creates and removes**: an agent may be sitting in this branch's real
    worktree, and rewriting that underneath them is not a thing a merge queue
    gets to do.

    The push happens from inside that worktree, before it goes away, so the
    rebased commits are still referenced by something when they are sent. The
    lease is the head the pull request had when this branch's turn began, so a
    push from anywhere else in the meantime is refused rather than overwritten.

    Whether it pushed at all is [`push_needed`], which the caller asks again
    rather than being told — it is pure, and one answer is easier to trust than
    two.
    """
    with tempfile.TemporaryDirectory(prefix="scorsese-queue-") as tmp:
        work = os.path.join(tmp, "wt")
        made = talk.git("worktree", "add", "--detach", work, head, cwd=root)
        if made.returncode != 0:
            return None, [f"could not check {branch} out: {made.stderr.strip()}"]
        try:
            done = talk.git("rebase", "origin/main", cwd=work)
            if done.returncode != 0:
                unmerged = talk.git(
                    "diff", "--name-only", "--diff-filter=U", cwd=work
                ).stdout
                talk.git("rebase", "--abort", cwd=work)
                paths = conflicts(unmerged)
                return None, [
                    f"{branch} conflicts with `main`.",
                    *(
                        [f"Conflicted: {', '.join(paths)}."]
                        if paths
                        else ["The rebase stopped; git did not name a path."]
                    ),
                    "Handed back unresolved. A machine that cannot say why the"
                    " code is shaped as it is does not get to pick a side —"
                    " `SYNTH_VERSION` is the standing example, where the answer"
                    " is the next number and neither side is right.",
                ]
            fresh = talk.git("rev-parse", "HEAD", cwd=work).stdout.strip()
            clashes = collisions(head, work)
            if clashes:
                return None, [
                    f"{branch} rebases cleanly but collides on a number.",
                    *clashes,
                    "Handed back unpushed: CI would only find it ten minutes on.",
                ]
            if push_needed(head, fresh):
                pushed = talk.git(
                    "push",
                    f"--force-with-lease=refs/heads/{branch}:{head}",
                    "origin",
                    f"HEAD:refs/heads/{branch}",
                    cwd=work,
                )
                if pushed.returncode != 0:
                    return None, [
                        f"the force-push of {branch} was refused:"
                        f" {pushed.stderr.strip()}",
                        "The lease held the head this queue started from, so"
                        " something else has pushed since. Handed back.",
                    ]
            return fresh, []
        finally:
            talk.git("worktree", "remove", "--force", work, cwd=root)


def collisions(head: str, work: str) -> list[str]:
    """[`numbering`] over the rebased tree in `work`; `head` is the old head."""

    def listing(*args: str) -> list[str]:
        # The trailing slash matters: without it `ls-tree` names the directory
        # itself rather than the files in it.
        return talk.git(*args, "--", f"{MIGRATIONS}/", cwd=work).stdout.split()

    def version(rev: str) -> int | None:
        shown = talk.git("show", f"{rev}:{SCHEMA_FILE}", cwd=work)
        return numbering.schema_version(shown.stdout if shown.returncode == 0 else None)

    fork = talk.git("merge-base", head, "origin/main", cwd=work).stdout.strip()
    return numbering.numbering(
        listing("ls-tree", "--name-only", "origin/main"),
        listing("diff", "--name-only", "--diff-filter=A", "origin/main", "HEAD"),
        (version(fork), version(head), version("HEAD"), version("origin/main")),
    )


def on_tip(sha: str, root: str) -> bool:
    """Whether `sha` already contains `origin/main`'s tip (fetched by the caller).

    The sha is fetched by itself first: a head Dependabot has just pushed is not
    on any ref this checkout has fetched yet.
    """
    talk.git("fetch", "--quiet", "origin", sha, cwd=root)
    return talk.git("merge-base", "--is-ancestor", "origin/main", sha, cwd=root).returncode == 0


def bot_head(
    number: int, head: str, root: str, opts: argparse.Namespace
) -> tuple[str | None, list[str]]:
    """Dependabot's own rebase, waited for: a head on `main`'s tip, or why not.

    Never force-pushed from here (see the module doc). Asked once with
    [`BOT_REBASE`], then polled until a new head on `main`'s tip appears or
    `--deadline` runs out — the second is a hand-back, remembered by head like
    any other, so the watch does not ask again until Dependabot pushes.
    """
    if on_tip(head, root):
        return head, []
    # Not `mergeable.gh`: if the comment fails, the wait below still answers.
    subprocess.run(
        ["gh", "pr", "comment", str(number), "--body", BOT_REBASE],
        capture_output=True,
        check=False,
    )
    began = time.monotonic()
    while time.monotonic() - began < opts.deadline * 60:
        time.sleep(opts.poll)
        seen = talk.look(number).get("headRefOid", "")
        if seen != head and on_tip(seen, root):
            return seen, []
        talk.say(f"#{number}: waiting for Dependabot to rebase {head[:7]}.")
    return None, [
        f"Dependabot did not rebase {head[:7]} onto `main` within"
        f" {opts.deadline:.0f} minutes."
    ]
