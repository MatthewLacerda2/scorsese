#!/usr/bin/env python3
"""Render `cargo llvm-cov --json` output as the Markdown an agent reads.

Presentation is the point of this file. llvm-cov's own text report is a
twelve-column table with a row per file: correct, and nobody reads it. A signal
nobody reads is not a signal, so what comes out here is ordered by what someone
would act on — the headline, then which crate is thin, then the files with
functions no test calls at all.

Run it locally against exactly what CI produces:

    cargo llvm-cov --workspace --exclude-from-report scorsese-golden \\
        --json --output-path coverage.json
    python3 .github/scripts/coverage-summary.py coverage.json

Python and not jq because this has to run on a developer's machine as readily
as on a runner, and python3 is the one of the two that is always already there.

This never exits non-zero over a coverage number, and there is deliberately no
threshold to compare against. Coverage audits quality; it does not prove
correctness. See the `coverage` job in `.github/workflows/ci.yml`.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# How many rows the "least covered" table shows. Long enough to be a worklist,
# short enough that the summary stays skimmable.
WORST_N = 12


def pct(part: float, whole: float) -> float:
    """Percentage, with an empty denominator counting as fully covered."""
    return 100.0 if whole == 0 else 100.0 * part / whole


def bar(percent: float, width: int = 10) -> str:
    """A fixed-width meter, so crates compare at a glance and not by reading."""
    filled = int(percent / 100 * width + 0.5)
    return "█" * filled + "░" * (width - filled)


def crate_of(path: str) -> str:
    """`crates/render/src/run.rs` -> `crates/render`. The unit people think in."""
    parts = path.split("/")
    return "/".join(parts[:2]) if len(parts) > 2 else path


class Totals:
    """Covered-of-total counts for lines, functions and regions."""

    def __init__(self) -> None:
        self.lines = self.lines_hit = 0
        self.funcs = self.funcs_hit = 0

    def add(self, summary: dict) -> "Totals":
        self.lines += summary["lines"]["count"]
        self.lines_hit += summary["lines"]["covered"]
        self.funcs += summary["functions"]["count"]
        self.funcs_hit += summary["functions"]["covered"]
        return self

    @property
    def untested_fns(self) -> int:
        return self.funcs - self.funcs_hit


def load(report: Path, root: str) -> tuple[dict, list[tuple[str, dict]]]:
    """The report's totals, and every file in it keyed by repo-relative path."""
    data = json.loads(report.read_text())["data"][0]
    files = []
    for entry in data["files"]:
        name = entry["filename"]
        # Paths are absolute inside the runner; report them relative to the
        # repo root so the output means the same thing wherever it was made.
        if name.startswith(root):
            name = name[len(root) :]
        files.append((name.lstrip("/"), entry["summary"]))
    return data["totals"], files


def headline(totals: dict) -> list[str]:
    lines, funcs, regions = totals["lines"], totals["functions"], totals["regions"]
    missed = funcs["count"] - funcs["covered"]
    return [
        "## Coverage",
        "",
        f"### {lines['percent']:.1f}% of lines"
        f" · {funcs['percent']:.1f}% of functions"
        f" · {regions['percent']:.1f}% of regions",
        "",
        f"{lines['covered']:,} of {lines['count']:,} lines executed."
        f" **{missed} function{'' if missed == 1 else 's'} no test calls at all.**",
        "",
    ]


def by_crate(files: list[tuple[str, dict]]) -> list[str]:
    crates: dict[str, Totals] = {}
    for name, summary in files:
        crates.setdefault(crate_of(name), Totals()).add(summary)

    rows = ["### By crate", "", "| Crate | Lines | Functions | Untested fns |",
            "| --- | --- | --: | --: |"]
    worst_first = sorted(crates.items(), key=lambda kv: pct(kv[1].lines_hit, kv[1].lines))
    for name, t in worst_first:
        line_pct = pct(t.lines_hit, t.lines)
        rows.append(
            f"| `{name}` | `{bar(line_pct)}` {line_pct:.1f}%"
            f" | {pct(t.funcs_hit, t.funcs):.1f}% | {t.untested_fns or ''} |"
        )
    return rows + [""]


def worst_files(files: list[tuple[str, dict]]) -> list[str]:
    """Files with functions nothing calls — the actionable half of the report."""
    gaps = []
    for name, summary in files:
        untested = summary["functions"]["count"] - summary["functions"]["covered"]
        if untested:
            gaps.append((untested, summary["lines"]["percent"], name))
    if not gaps:
        return ["### Least-covered files", "",
                "Every function in the report is called by at least one test.", ""]

    gaps.sort(key=lambda g: (-g[0], g[1]))
    rows = [
        "### Files with functions no test calls",
        "",
        "| File | Lines | Untested fns |",
        "| --- | --: | --: |",
    ]
    for untested, line_pct, name in gaps[:WORST_N]:
        rows.append(f"| `{name}` | {line_pct:.1f}% | {untested} |")
    if len(gaps) > WORST_N:
        rows.append(f"| … and {len(gaps) - WORST_N} more | | |")
    return rows + [""]


FOOTER = (
    "<sub><b>This is a signal, not a gate.</b> There is no threshold and it can"
    " never fail a build. Coverage audits quality; it does not prove"
    " correctness — an executed line is not an asserted-on line, and this"
    " repository's strongest tests (loudness over a window, pixels against a"
    " reference) barely move the number. Read it for the one thing it says"
    " reliably: code no test reaches at all. `crates/golden` and `tools/lint`"
    " are excluded, with reasons, in the `coverage` job of"
    " <code>.github/workflows/ci.yml</code>.</sub>"
)


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: coverage-summary.py <coverage.json> [repo-root]")
    root = sys.argv[2] if len(sys.argv) > 2 else str(Path.cwd())
    totals, files = load(Path(sys.argv[1]), root.rstrip("/"))
    out = headline(totals) + by_crate(files) + worst_files(files) + [FOOTER]
    print("\n".join(out))


if __name__ == "__main__":
    main()
