#!/usr/bin/env python3
"""`mutants/exclusions.py [MUTANTS_TOML]` — hold `exclude_re` to its own rules.

`.cargo/mutants.toml` says what an equivalent-mutant entry must be, and
`ci-merge` says it again; until #1229 nothing checked either, and 19 of 46
entries had drifted from it. Three rules, each one a way an entry has gone
wrong before:

- **It carries a written reason**: a comment directly above it (entries in a
  run share the comment above the run). An exclusion nobody can read the
  reason for cannot be re-examined when the code under it changes.
- **Its metacharacters are escaped.** An entry is a regex, and a mutation of
  an operator puts `|`, `+`, `*` or `?` in the name: an unescaped `||` is an
  alternation with an empty branch, which matches every mutant there is
  (#363). Inside a `[…]` class they are literal, and left alone.
- **It names its line**: `file\\.rs:LINE:COL: …`. A name alone covers every
  mutation of that shape in every function sharing a prefix — `in line_len`
  silently excluded `delay.rs`'s `line_length` too — and a line that moves
  puts the mutant back in the report, which is the right direction to fail.

Exits 1 and names each entry that breaks one. `make scripts` runs its tests,
which run it over the real file.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

#: The file it reads by default, from the repository root.
DEFAULT = Path(__file__).resolve().parents[3] / ".cargo" / "mutants.toml"

#: An entry's line: the file, escaped, then its line and column.
QUALIFIED = re.compile(r"\\\.rs:\d+:\d+: ")

#: Characters that are an operator in a mutant's name and a quantifier or an
#: alternation in a regex.
METACHARACTERS = "|+*?"

#: One entry line of the array: a TOML literal string and a comma.
ENTRY = re.compile(r"^\s*'(?P<pattern>[^']*)',\s*$")


def unescaped(pattern: str) -> list[str]:
    """The metacharacters in `pattern` neither escaped nor inside a class."""
    found, escaped, in_class = [], False, False
    for char in pattern:
        if escaped:
            escaped = False
        elif char == "\\":
            escaped = True
        elif char == "[":
            in_class = True
        elif char == "]":
            in_class = False
        elif char in METACHARACTERS and not in_class:
            found.append(char)
    return found


def entries(text: str) -> list[tuple[int, str, bool]]:
    """Each `exclude_re` entry: its line number, its pattern, and whether a
    comment sits directly above it or above the run of entries it is in."""
    lines = text.splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith("exclude_re = ["))
    found, commented = [], False
    for number, line in enumerate(lines[start + 1 :], start + 2):
        stripped = line.strip()
        if stripped == "]":
            break
        match = ENTRY.match(line)
        if match:
            found.append((number, match.group("pattern"), commented))
        elif stripped.startswith("#"):
            commented = True
            continue
        # A blank line ends a run; an entry continues it.
        if not stripped:
            commented = False
    return found


def problems(text: str) -> list[str]:
    """Every entry that breaks a rule, as `line N: 'pattern': why`."""
    found = []
    for number, pattern, commented in entries(text):
        why = []
        if not commented:
            why.append("no comment above it saying why it is equivalent")
        stray = unescaped(pattern)
        if stray:
            why.append(f"unescaped {' '.join(sorted(set(stray)))}: escape it, it is a regex")
        if not QUALIFIED.search(pattern):
            why.append(r"no line: write it as `file\.rs:LINE:COL: …`, from `cargo mutants --list`")
        found.extend(f"line {number}: '{pattern}': {reason}" for reason in why)
    return found


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    found = problems(path.read_text(encoding="utf-8"))
    for problem in found:
        print(f"exclusions: {path}: {problem}", file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    raise SystemExit(main())
