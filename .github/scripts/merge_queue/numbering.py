"""The two numbers a clean rebase can still collide on (#729). Pure, no git.

[`rebase.collisions`] reads the tree; this decides.
"""

from __future__ import annotations

import re


# The two numbers a clean rebase can still collide on (#729). Both are chosen
# by counting up from `main`, so two branches cut from the same `main` pick the
# same one, and git sees no conflict: different migration filenames, or the
# identical `SCHEMA_VERSION` edit on both sides. CI finds either a full round
# later; a filename listing and a `git show` find it before the push.
MIGRATIONS = "crates/server/migrations"
SCHEMA_FILE = "crates/core/src/project.rs"
SCHEMA_LINE = re.compile(r"pub const SCHEMA_VERSION: u32 = (\d+);")


def migration_number(path: str) -> int | None:
    """The `0017` of `…/0017_library_midi.sql`, or None for anything else."""
    name = path.rsplit("/", 1)[-1]
    head = name.split("_", 1)[0]
    return int(head) if name.endswith(".sql") and head.isdigit() else None


def schema_version(source: str | None) -> int | None:
    """`SCHEMA_VERSION` as `project.rs` declares it, or None if it does not."""
    found = SCHEMA_LINE.search(source or "")
    return int(found.group(1)) if found else None


def numbering(
    on_main: list[str],
    added: list[str],
    versions: tuple[int | None, int | None, int | None, int | None],
) -> list[str]:
    """Why this rebased branch has taken a number `main` already holds, if it has.

    `on_main` is the migration paths on `origin/main`; `added` the ones the
    rebased branch adds. `versions` is `SCHEMA_VERSION` at the branch's fork
    point, its old head, the rebased head and `origin/main`. Pure, so the
    whole judgement is tested without git, like [`push_needed`]. The queue
    only names the number; renumbering is the author's, because the migration
    step and the fixtures move with it.

    A Markdown-only branch or a Dependabot bump adds no migration and leaves
    `SCHEMA_VERSION` alone, so it passes here without being told to skip.
    """
    taken = {migration_number(p) for p in on_main} - {None}
    mine = sorted(
        (n, p) for p in added if (n := migration_number(p)) is not None
    )
    nums = [n for n, _ in mine]
    clash = [p for n, p in mine if n in taken or nums.count(n) > 1]
    lines = []
    if clash:
        lines.append(
            f"Migration number already taken: {', '.join(clash)}; renumber"
            f" from {max(taken | {0}) + 1}."
        )
    fork, head, rebased, main = versions
    if head != fork and rebased is not None and rebased == main:
        lines.append(
            f"`SCHEMA_VERSION` {rebased} is already `main`'s: a sibling's bump"
            f" took it. Renumber to {rebased + 1}, with its migration step and"
            " fixtures."
        )
    return lines
