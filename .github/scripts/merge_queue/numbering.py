"""The numbers a clean rebase can still collide on (#729, #1216). Pure, no git.

[`rebase.collisions`] reads the tree; this decides.
"""

from __future__ import annotations

import re


# The numbers a clean rebase can still collide on (#729). Each is chosen by
# counting up from `main`, so two branches cut from the same `main` pick the
# same one, and git sees no conflict: different migration filenames, or the
# identical version edit on both sides. CI finds a migration clash a full round
# later and a `SCHEMA_VERSION` one maybe; a `SYNTH_VERSION` one it never finds,
# since the second branch's sound change simply ships under the first's bump
# and every bake of the first's number goes stale silently (#1216). A filename
# listing and a `git show` find all three before the push.
MIGRATIONS = "crates/server/migrations"
SCHEMA_FILE = "crates/core/src/project/mod.rs"
SCHEMA_LINE = re.compile(r"pub const SCHEMA_VERSION: u32 = (\d+);")
SYNTH_FILE = "crates/zimmer/src/lib.rs"
SYNTH_LINE = re.compile(r"pub const SYNTH_VERSION: u32 = (\d+);")

# What a renumbering carries with it, so the line handed back is the whole job.
_CARRIES = {
    "SCHEMA_VERSION": "with its migration step and fixtures",
    "SYNTH_VERSION": (
        "with its row in the constant's table and its entry in"
        " `crates/zimmer/SYNTH_VERSION.md`"
    ),
}

Versions = tuple[int | None, int | None, int | None, int | None]


def migration_number(path: str) -> int | None:
    """The `0017` of `…/0017_library_midi.sql`, or None for anything else."""
    name = path.rsplit("/", 1)[-1]
    head = name.split("_", 1)[0]
    return int(head) if name.endswith(".sql") and head.isdigit() else None


def schema_version(source: str | None) -> int | None:
    """`SCHEMA_VERSION` as `project/mod.rs` declares it, or None if it does not."""
    found = SCHEMA_LINE.search(source or "")
    return int(found.group(1)) if found else None


def synth_version(source: str | None) -> int | None:
    """`SYNTH_VERSION` as zimmer's `lib.rs` declares it, or None if it does not."""
    found = SYNTH_LINE.search(source or "")
    return int(found.group(1)) if found else None


def _taken(name: str, versions: Versions) -> str | None:
    """Why this branch's bump of `name` is one `main` already made, if it is.

    `versions` is the constant at the fork point, the old head, the rebased
    head and `origin/main`. A branch that bumped (old head differs from the
    fork) and rebased onto a `main` holding the very same number took a
    sibling's bump: the identical edit, so git merged it silently.
    """
    fork, head, rebased, main = versions
    if head != fork and rebased is not None and rebased == main:
        return (
            f"`{name}` {rebased} is already `main`'s: a sibling's bump took it."
            f" Renumber to {rebased + 1}, {_CARRIES[name]}."
        )
    return None


def numbering(
    on_main: list[str],
    added: list[str],
    versions: Versions,
    synth: Versions = (None, None, None, None),
) -> list[str]:
    """Why this rebased branch has taken a number `main` already holds, if it has.

    `on_main` is the migration paths on `origin/main`; `added` the ones the
    rebased branch adds. `versions` is `SCHEMA_VERSION` at the branch's fork
    point, its old head, the rebased head and `origin/main`; `synth` is
    `SYNTH_VERSION` at the same four, the default reading as no evidence for
    a caller that has none. Pure, so the
    whole judgement is tested without git, like [`push_needed`]. The queue
    only names the number; renumbering is the author's, because the migration
    step and the fixtures move with it.

    A Markdown-only branch or a Dependabot bump adds no migration and leaves
    both versions alone, so it passes here without being told to skip.
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
    for name, seen in (("SCHEMA_VERSION", versions), ("SYNTH_VERSION", synth)):
        if line := _taken(name, seen):
            lines.append(line)
    return lines
