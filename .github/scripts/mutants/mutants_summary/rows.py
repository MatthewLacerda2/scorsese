"""One survivor, as a table row that cannot be mistaken for its neighbour."""

from __future__ import annotations

import re


# ---------------------------------------------------------------------------
# One survivor, as a row that cannot be mistaken for its neighbour
# ---------------------------------------------------------------------------
#
# A row used to say only `file:line` and what the code was replaced *with*,
# and on a line holding several operators that is ambiguous. #596's report
# listed `cpu.rs:152 demultiply — replaced with *` and `replaced with %`; the
# follow-up read both as mutations of the division, and they were `+`→`*` at
# column 52 and the `/` of `alpha / 2` at column 60 (#616). cargo-mutants had
# recorded all of it — the column in the span, the original in the mutant's
# `name` — and the rendering dropped it.
#
# The original is read from `name` because it is the only place cargo-mutants
# 27.1.0 writes it: `file:line:column: replace / with % in demultiply`. The
# location prefix and the function suffix are stripped by exact match against
# fields the same record carries, never by a pattern that guesses where they
# end, and anything whose shape is not recognised is printed as cargo-mutants
# wrote it — a description in its own words is still unambiguous next to a
# column.


def where(mutant: dict) -> str:
    """`file:line:column`, or `file:line` where no column was recorded."""
    start = mutant["span"]["start"]
    column = f":{start['column']}" if "column" in start else ""
    return f"{mutant['file']}:{start['line']}{column}"


def function_name(mutant: dict) -> str:
    return (mutant.get("function") or {}).get("function_name", "")


def described(mutant: dict) -> str | None:
    """What cargo-mutants says it did, without the location and the function."""
    name = mutant.get("name")
    prefix = where(mutant) + ": "
    if not name or not name.startswith(prefix):
        return None
    what = name[len(prefix) :]
    suffix = f" in {function_name(mutant)}"
    return what[: -len(suffix)] if function_name(mutant) and what.endswith(suffix) else what


def code(text: str) -> str:
    """`text` as a code span that survives a Markdown table cell.

    A pipe ends the cell even inside backticks, and `||` is an operator
    cargo-mutants mutates; GitHub reads `\\|` as a literal pipe in both places.
    """
    return "`" + text.replace("|", "\\|") + "`"


def mutation(mutant: dict) -> str:
    """The change, as `original → replacement` wherever that is its shape."""
    new = mutant["replacement"]
    what = described(mutant)
    if what is None:
        return f"replaced with {code(new)}"
    if mutant.get("genre") == "FnValue":
        return f"replace {code(function_name(mutant))} body with {code(new)}"
    if what.startswith("replace ") and what.endswith(f" with {new}"):
        original = what[len("replace ") : -len(f" with {new}")]
        guard = "match guard "
        if original.startswith(guard):
            return f"match guard {code(original[len(guard):])} → {code(new)}"
        return f"{code(original)} → {code(new)}"
    if mutant.get("genre") == "UnaryOperator" and what.startswith("delete "):
        return f"delete {code(what[len('delete ') :])}"
    if what.startswith("delete match arm "):
        return f"delete match arm {code(what[len('delete match arm ') :])}"
    field = STRUCT_FIELD.match(what)
    if field:
        return f"delete field {code(field['field'])} from {code(field['struct'] + ' { … }')}"
    return what.replace("|", "\\|")


# `delete field media from struct Asset expression`: a field dropped from a
# struct literal that has a `..base`, so the field falls back to the base's
# value — a survivor means nothing told the two apart.
STRUCT_FIELD = re.compile(r"^delete field (?P<field>\S+) from struct (?P<struct>\S+) expression$")


def row(mutant: dict) -> str:
    fn = code(function_name(mutant)) if function_name(mutant) else "—"
    return f"| {code(where(mutant))} | {fn} | {mutation(mutant)} |"
