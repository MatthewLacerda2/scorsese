#!/usr/bin/env python3
"""`mergeable.py N` — say whether pull request N has really been checked.

Exits 0 when CI genuinely ran on the head commit and passed, and non-zero with
a reason otherwise. It is the last thing the merge routine does before
`gh pr merge`, and it exists because "the checks look green" and "the checks
ran" are not the same claim.

The failure it was written for is #153. Push to a draft pull request and mark
it ready seconds later, and the push produces a run that correctly skips every
job — the pull request was a draft, and `ci.yml` keeps CI quiet on drafts
deliberately. The `ready_for_review` that follows sometimes produces no run at
all. What is left is a pull request in the ready state whose only recorded run
is a skipped one: `gh pr checks` reports every check as SKIPPED, GitHub calls
the branch mergeable, and nothing has compiled the code. That is worse than a
red gate, because an absent check is wearing the costume of a passing one.

Branch protection is the obvious alternative and probably does not work here.
GitHub generally treats a skipped job as satisfying a required check, and our
jobs skip by design on drafts — so a required-checks rule would look at exactly
the run this is meant to catch and call it satisfied. This asks the question
directly instead, and it is the only remedy that also catches a run that goes
missing for some future reason nobody has predicted yet.

Run it against a live pull request:

    python3 .github/scripts/mergeable.py 171

Python and not Rust because it is a few `gh` calls and a decision, and because
`.github/scripts/` is already where that kind of thing lives. The decision
itself is [`judge`], which is pure and takes plain dictionaries, so the whole
of it is tested without a network.
"""

from __future__ import annotations

import json
import subprocess
import sys

# The workflow that gates a merge. A run of anything else — a scheduled job, a
# future workflow — is not the one being asked about, and counting it would be
# the same mistake in a new costume.
WORKFLOW = "CI"


def gh(*args: str) -> object:
    """`gh` with `--json`-shaped output, parsed. Fatal if `gh` itself fails."""
    done = subprocess.run(
        ["gh", *args], capture_output=True, text=True, check=False
    )
    if done.returncode != 0:
        sys.exit(f"mergeable: gh {' '.join(args)}: {done.stderr.strip()}")
    return json.loads(done.stdout)


def latest_run(runs: list[dict], sha: str) -> dict | None:
    """The most recent [`WORKFLOW`] run for `sha`, or `None` if there is none.

    Most recent and not first-found: a branch that was marked ready, pushed to
    and marked ready again can have several, and the one that decides is the
    one that ran last. Runs come back newest-first, so this is the first match,
    but saying so in one place beats relying on the order everywhere.
    """
    for run in runs:
        if run.get("name") == WORKFLOW and run.get("head_sha") == sha:
            return run
    return None


def judge(pull: dict, run: dict | None, jobs: list[dict]) -> tuple[bool, list[str]]:
    """Whether this pull request may be merged, and why not when it may not.

    Pure, and the whole of the decision. `pull` is `gh pr view --json`, `run` is
    the run [`latest_run`] picked, and `jobs` are that run's jobs.
    """
    sha = pull.get("headRefOid", "")
    short = sha[:7]

    if pull.get("isDraft"):
        return False, [
            "the pull request is a draft.",
            "CI does not run on drafts, so nothing here has checked it. A draft"
            " makes no claim to pass; mark it ready and let CI answer.",
        ]

    if run is None:
        return False, [
            f"no {WORKFLOW} run exists for the head commit {short}.",
            "This is #153: marking a pull request ready right after a push can"
            " lose the run entirely. The checks are not green, they are absent.",
            "Force one with an empty commit, or draft and ready it again with a"
            " pause in between.",
        ]

    if run.get("status") != "completed":
        return False, [
            f"the {WORKFLOW} run for {short} is {run.get('status')}.",
            "Wait for it. A run in flight has not passed yet.",
        ]

    conclusion = run.get("conclusion")
    if conclusion == "skipped":
        return False, [
            f"the {WORKFLOW} run for {short} skipped every job.",
            "That is what a run against a draft looks like, so this is a run"
            " that was created before the pull request was ready and never"
            " replaced — #153 exactly. Nothing compiled this commit.",
        ]
    if conclusion != "success":
        return False, [
            f"the {WORKFLOW} run for {short} concluded {conclusion}.",
            f"See {run.get('html_url', 'the run')}.",
        ]

    passed = [job for job in jobs if job.get("conclusion") == "success"]
    if not passed:
        return False, [
            f"the {WORKFLOW} run for {short} says success but no job in it ran.",
            "A run that checked nothing is not a run that passed.",
        ]

    lines = [f"{WORKFLOW} passed on {short}: {len(passed)} jobs ran."]
    skipped = [job["name"] for job in jobs if job.get("conclusion") == "skipped"]
    if skipped:
        # The app gate skips by design when a branch touches nothing under
        # `app/`. Naming what did not run is the difference between this and
        # the report it was written to distrust.
        lines.append(f"Skipped, and expected to be: {', '.join(skipped)}.")
    return True, lines


def main() -> int:
    if len(sys.argv) != 2:
        sys.exit("usage: mergeable.py PULL_REQUEST_NUMBER")
    number = sys.argv[1]

    repo = gh("repo", "view", "--json", "nameWithOwner")["nameWithOwner"]
    pull = gh("pr", "view", number, "--json", "isDraft,headRefOid,number")
    sha = pull["headRefOid"]

    runs = gh("api", f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100")
    run = latest_run(runs.get("workflow_runs", []), sha)
    jobs = []
    if run is not None:
        # Asked of the run itself rather than of the pull request, because
        # `gh pr checks` blends jobs from every run on the branch — which is
        # how a skipped run and a green one can appear as one passing report.
        listing = gh("api", f"repos/{repo}/actions/runs/{run['id']}/jobs")
        jobs = listing.get("jobs", [])

    ok, lines = judge(pull, run, jobs)
    head, *rest = lines
    where = sys.stdout if ok else sys.stderr
    print(f"mergeable: #{number}: {head}", file=where)
    for line in rest:
        print(f"  {line}", file=where)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
