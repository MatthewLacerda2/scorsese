#!/usr/bin/env python3
"""The edit behind every survivor, as one file of unified diffs.

The report says *which* mutations survived; the diff is usually the fastest way
to see what a survivor means (`docs/mutation-testing.md`, *Triaging a
survivor*, step 1). Locally that is `mutants.out/diff/`. A run on GitHub's
runners has no such directory to hand back — each shard wrote its own and the
shards are gone — so `mutants-on-request.yml` keeps what the plan already
listed: `cargo mutants --list --json` carries every planned mutant's diff, and
the survivors are looked up in it by name.

    mutants-diffs.py merged.json planned.json > survivors.diff

A survivor the list does not know is named with a note rather than skipped,
because a diff file one entry short reads as one survivor fewer.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def survivors(run: dict) -> list[dict]:
    """The missed mutants, in source order, as the report lists them."""
    found = [
        o["scenario"]["Mutant"]
        for o in run.get("outcomes", [])
        if isinstance(o.get("scenario"), dict) and o.get("summary") == "MissedMutant"
    ]
    return sorted(found, key=lambda m: (m["file"], m["span"]["start"]["line"], m["name"]))


def render(run: dict, planned: list[dict]) -> str:
    """One `# name` line and its diff per survivor; empty when none survived."""
    diffs = {m["name"]: m.get("diff", "") for m in planned}
    out = []
    for mutant in survivors(run):
        out.append(f"# {mutant['name']}")
        diff = diffs.get(mutant["name"])
        out.append(diff.rstrip("\n") if diff else "# (not in the plan's list; no diff kept)")
        out.append("")
    return "\n".join(out)


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: mutants-diffs.py OUTCOMES PLANNED")
    run = json.loads(Path(sys.argv[1]).read_text())
    listed = Path(sys.argv[2]).read_text().strip()
    sys.stdout.write(render(run, json.loads(listed) if listed else []))


if __name__ == "__main__":
    main()
