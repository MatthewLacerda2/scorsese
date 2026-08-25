#!/usr/bin/env python3
"""Has the mutated surface collapsed?

The failure this exists for is #363 and #365, which were the same bug twice.
`exclude_re` in `.cargo/mutants.toml` is a list of **regular expressions**, and
three entries were written as the plain line `cargo mutants --list` prints. One
of them was `replace || with && in unit`: an alternation with an empty branch,
an empty branch matches every string there is, and so the entry excluded the
entire surface. Nothing said so. The report read *nothing to report* — which is
also what it says on a healthy branch that happens to touch no mutated lines —
`make gates` went green, and the signal was dead across ten merged pull
requests until an agent ran `--list` by hand for an unrelated reason.

So this compares the size of the surface against a **floor**, recorded as a
`surface-floor:` comment in `.cargo/mutants.toml` beside the count it is a
floor under. A floor and not an exact number: writing code adds mutants and
must never break this check, while deleting the surface is precisely what it
is here to notice.

The check is free. `cargo mutants --list` builds nothing and runs nothing — it
parses and prints, in well under a second — which is why it can sit in front of
every run rather than being a thing somebody remembers to do.

This script does no subprocess work of its own: it is handed the list its
caller already captured, so it can be tested on a list that no cargo produced.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

#: The line in `.cargo/mutants.toml` that carries the floor. It is a comment
#: and not a config key because cargo-mutants rejects an unknown field outright
#: — `surface_floor = 3000` in that file makes every run print a parse error and
#: list nothing, which is the very failure this guards against.
FLOOR = re.compile(r"^#\s*surface-floor:\s*(\d+)\s*$", re.MULTILINE)

#: Where the floor lives when the caller does not say.
DEFAULT_CONFIG = Path(__file__).resolve().parents[2] / ".cargo" / "mutants.toml"

USAGE = "usage: mutation-surface.py LIST_FILE [MUTANTS_TOML]"

COLLAPSED = """\
The mutation surface has collapsed: {count} mutants, and the floor is {floor}.

Nothing about this is a survivor list or a test gap -- the instrument itself is
reporting on almost nothing, so whatever it says next is worthless. The usual
cause is an `exclude_re` entry in {config}: that field holds regular
expressions, not the lines `cargo mutants --list` prints, and an unescaped `|`
is an alternation with an empty branch that matches every mutant there is. See
#363, and the paragraph above `exclude_re` in that file.

    cargo mutants --list | head          # what is left of the surface
    git diff {config}

If the surface genuinely shrank -- a crate left `examine_globs` on purpose --
then lower the `surface-floor:` line in {config} in the same commit that
shrinks it, with the reason.\
"""


def floor_from(config: Path) -> int:
    """The floor recorded in `.cargo/mutants.toml`."""
    found = FLOOR.search(config.read_text(encoding="utf-8"))
    if found is None:
        raise SystemExit(
            f"{config} has no `# surface-floor: N` line, so there is nothing to"
            " check the surface against. It was deleted, not satisfied: put it"
            " back next to the mutant count it is a floor under."
        )
    return int(found.group(1))


def counted(listing: Path) -> int:
    """How many mutants `cargo mutants --list` printed.

    Blank lines do not count, so a trailing newline is not a mutant and a list
    of nothing at all counts as zero rather than as one.
    """
    try:
        text = listing.read_text(encoding="utf-8")
    except OSError as why:
        raise SystemExit(f"cannot read the mutant list at {listing}: {why}") from why
    return sum(1 for line in text.splitlines() if line.strip())


def main(argv: list[str]) -> int:
    """Compare the list against the floor and say which way it came out."""
    if not 1 <= len(argv) <= 2:
        raise SystemExit(USAGE)

    config = Path(argv[1]) if len(argv) == 2 else DEFAULT_CONFIG
    floor = floor_from(config)
    count = counted(Path(argv[0]))

    if count < floor:
        print(COLLAPSED.format(count=count, floor=floor, config=config), file=sys.stderr)
        return 1

    print(f"mutation surface: {count} mutants, floor {floor}. Intact.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
