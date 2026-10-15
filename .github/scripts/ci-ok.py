#!/usr/bin/env python3
"""`CI ok`: one check that stands for every job in `ci.yml` (#1351).

The `main` ruleset requires this one context rather than each job by name, so
a job added to `ci.yml` is gated the moment it is listed under `CI ok`'s
`needs:`, with nothing to change in Terraform. It reads `toJSON(needs)` from
the environment and fails when any needed job's result is `failure` or
`cancelled`.

`skipped` passes, and that is deliberate: it is `changes` saying a
Markdown-only diff touched nothing a job checks. A job that never ran because
something it needs failed is `skipped` too, but the failure it waited on is in
the same set and refuses on its own.

`cancelled` refuses because a cancelled job judged nothing, and `CI ok` runs
under `always()` precisely so that it still reports then: a required check
that skipped on a cancelled run would read to GitHub as satisfied.
"""

from __future__ import annotations

import json
import os
import sys

# The results that mean a job did not pass. `success` and `skipped` are the two
# that do; anything else GitHub ever adds is read as not passing, too.
PASSING = ("success", "skipped")


def refused(needs: dict[str, dict]) -> list[str]:
    """Every needed job that did not pass, as `name: result`, in `needs`' order.

    An empty `needs` refuses: a `CI ok` that needs nothing vouches for nothing.
    """
    if not needs:
        return ["needs: nothing — `CI ok` must list every other job"]
    return [
        f"{name}: {job.get('result')}"
        for name, job in needs.items()
        if job.get("result") not in PASSING
    ]


def main() -> int:
    needs = json.loads(os.environ["NEEDS"])
    for name, job in needs.items():
        print(f"{name}: {job.get('result')}")
    bad = refused(needs)
    if bad:
        print(f"CI is not ok: {', '.join(bad)}", file=sys.stderr)
        return 1
    print("CI ok: every job passed, or had nothing of its own to check.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
