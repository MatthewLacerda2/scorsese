#!/usr/bin/env python3
"""Put a sharded mutation run back together as the one run it was.

`cargo mutants --shard k/n` divides the mutants of a diff across n jobs that
know nothing about each other, and each writes its own `outcomes.json`. This
reads those and prints the single document they add up to, which
`mutants-summary.py` then renders exactly as it renders an unsharded run: one
renderer, one report, and a reader who never has to know how many machines
produced it.

Merging is deliberately not the renderer's job. How a run was *executed* — on
one runner or on four — is a property of the job, and the report is a statement
about the code. Keeping the seam here is what lets the scheduled sweep, which
never shards, go on calling the renderer with the file cargo-mutants wrote.

Two things this will not do, both for the same reason: a report that quietly
described less than it was asked about is the failure #399 was filed over.

- **A shard that did not report is not silently dropped.** `--expect` says how
  many there should be, and a run short of that is stamped as one that did not
  finish — no `end_time` — which is what `mutants-summary.py` already reads as
  *this describes part of the work and not all of it*.
- **A file that cannot be parsed counts as a shard that did not report.** A
  shard killed part-way through a write leaves truncated JSON behind, and dying
  on it would turn a partial answer into no answer at all — which is the state
  this whole change exists to get out of.

What is printed is *not* a cargo-mutants artifact and does not pretend to be
one. It carries the fields the renderer reads and nothing else, because a
merged document that looked like the tool's own output would invite somebody to
read a field out of it that was never merged.

    python3 .github/scripts/mutants-merge.py --expect 3 shards/*/outcomes.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

#: The tallies a run is described by, summed across shards. Every one of them
#: counts mutants, so adding them is the same arithmetic the unsharded run does
#: internally — there is no rate or average here that summing would corrupt.
COUNTERS = ("total_mutants", "caught", "missed", "timeout", "unviable")


def shard(path: Path) -> dict | None:
    """One shard's outcomes, or `None` if it left nothing readable behind.

    Unreadable and absent are the same answer on purpose. Both mean *this shard
    did not report*, and the caller's response to either is identical: say so,
    and describe the run as incomplete rather than as smaller than it was.
    """
    try:
        loaded = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return loaded if isinstance(loaded, dict) else None


def merge(shards: list[dict | None], expected: int) -> dict:
    """The one run these shards add up to.

    `start_time` and `end_time` are the honesty in here. cargo-mutants stamps an
    end only on the way out, so the merged run gets one **only if every expected
    shard reported and every one of them finished** — a missing shard, or one
    killed under its time budget, leaves the end off and the renderer says the
    run was cut short. The timestamps are ISO-8601 in UTC with a fixed number of
    digits, so `min` and `max` over the strings order them correctly.
    """
    reported = [s for s in shards if s is not None]
    merged: dict = {name: sum(int(s.get(name, 0)) for s in reported) for name in COUNTERS}
    merged["outcomes"] = [o for s in reported for o in s.get("outcomes", [])]

    starts = [s["start_time"] for s in reported if s.get("start_time")]
    ends = [s["end_time"] for s in reported if s.get("end_time")]
    if starts:
        merged["start_time"] = min(starts)
    if ends and len(reported) == expected and len(ends) == len(reported):
        merged["end_time"] = max(ends)
    return merged


def parse_args(argv: list[str]) -> argparse.Namespace:
    """The argv contract the `mutants` job calls this on."""
    parser = argparse.ArgumentParser(
        prog="mutants-merge.py",
        description="Combine sharded cargo-mutants runs into one outcomes document.",
    )
    parser.add_argument(
        "shards", nargs="*", type=Path, help="each shard's outcomes.json, in any order"
    )
    parser.add_argument(
        "--expect",
        type=int,
        required=True,
        metavar="N",
        help="how many shards should have reported; a run short of this is"
        " marked as one that did not finish",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    """Merge, say what is missing, and print the document on stdout."""
    args = parse_args(argv)
    shards = [shard(path) for path in args.shards]

    for path, loaded in zip(args.shards, shards):
        if loaded is None:
            print(
                f"mutants-merge: {path} is missing or unreadable, so its shard"
                " reported nothing; the merged run is marked unfinished.",
                file=sys.stderr,
            )

    reported = sum(1 for loaded in shards if loaded is not None)
    if reported < args.expect:
        print(
            f"mutants-merge: {reported} of {args.expect} shards reported."
            " The report will say what it does not cover.",
            file=sys.stderr,
        )

    print(json.dumps(merge(shards, args.expect)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
