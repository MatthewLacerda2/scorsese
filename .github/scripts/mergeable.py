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

**One commit can carry several runs, and this reads all of them.** That is
#245, found when a draft push and a `ready_for_review` produced two runs a
second apart — one skipped, one green — and the earlier version of this script
picked whichever the API listed first. It happened to pick the skipped one and
refuse a pull request that had genuinely passed, which cost a wasted commit.
The direction that matters is the other one: nothing about "first in the list"
prefers a skipped run over a failing one, so the same flaw could have called a
commit mergeable while one of its runs was red. That is #153 again in a new
costume, produced by the tool written to prevent it.

So the rule is stated over the whole set rather than over a chosen member:
**no run for this commit failed, and at least one succeeded having actually
run its jobs.** A skipped run beside a real one then decides nothing, and a
failing run beside a green one refuses — which is the only safe reading of a
commit whose evidence disagrees with itself.

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


def runs_for(runs: list[dict], sha: str) -> list[dict]:
    """Every [`WORKFLOW`] run recorded against `sha`.

    All of them, and that is the whole of #245's fix. A commit routinely
    carries more than one — a draft push and the `ready_for_review` that
    follows are two, created seconds or even the same second apart — and
    picking one by list position means the verdict depends on an ordering
    GitHub never promised.

    Newest first is the order the API happens to return, and nothing here
    relies on it: [`judge`] reads the set.
    """
    return [
        run
        for run in runs
        if run.get("name") == WORKFLOW and run.get("head_sha") == sha
    ]


def ran_something(run: dict, jobs: dict[int, list[dict]]) -> bool:
    """Whether any job in this run actually concluded successfully.

    The question `conclusion == "success"` does not answer. A run whose every
    job was skipped can still conclude successfully, and that is exactly the
    shape of a run against a draft — success with nothing compiled.
    """
    return any(
        job.get("conclusion") == "success" for job in jobs.get(run.get("id"), [])
    )


def judge(
    pull: dict, runs: list[dict], jobs: dict[int, list[dict]]
) -> tuple[bool, list[str]]:
    """Whether this pull request may be merged, and why not when it may not.

    Pure, and the whole of the decision. `pull` is `gh pr view --json`, `runs`
    is every [`WORKFLOW`] run recorded against the head commit, and `jobs` maps
    a run's id to its jobs.

    Stated over the set rather than over a chosen run — see the module doc, and
    #245 for what picking one by list position did.
    """
    sha = pull.get("headRefOid", "")
    short = sha[:7]

    if pull.get("isDraft"):
        return False, [
            "the pull request is a draft.",
            "CI does not run on drafts, so nothing here has checked it. A draft"
            " makes no claim to pass; mark it ready and let CI answer.",
        ]

    if not runs:
        return False, [
            f"no {WORKFLOW} run exists for the head commit {short}.",
            "This is #153: marking a pull request ready right after a push can"
            " lose the run entirely. The checks are not green, they are absent.",
            "Force one with an empty commit, or draft and ready it again with a"
            " pause in between.",
        ]

    # A failure anywhere in the set refuses, and it is asked first. A commit
    # whose evidence disagrees with itself has exactly one safe reading, and a
    # green run sitting beside a red one does not make the red one go away.
    failed = [
        run
        for run in runs
        if run.get("status") == "completed"
        and run.get("conclusion") not in ("success", "skipped")
    ]
    if failed:
        run = failed[0]
        return False, [
            f"a {WORKFLOW} run for {short} concluded"
            f" {run.get('conclusion')}.",
            f"See {run.get('html_url', 'the run')}.",
            *(
                [
                    f"{len(runs)} runs exist for this commit; a green one beside"
                    " a red one is not a pass."
                ]
                if len(runs) > 1
                else []
            ),
        ]

    running = [run for run in runs if run.get("status") != "completed"]
    if running:
        return False, [
            f"a {WORKFLOW} run for {short} is {running[0].get('status')}.",
            "Wait for it. A run in flight has not passed yet.",
        ]

    # What is left is runs that all concluded success or skipped. At least one
    # of them has to have compiled something: a run whose every job skipped is
    # what a draft produces, and success with nothing built is the costume this
    # whole script exists to see through.
    real = [run for run in runs if ran_something(run, jobs)]
    if not real:
        return False, [
            f"no {WORKFLOW} run for {short} ran a single job.",
            "Every run against this commit skipped everything, which is what a"
            " run against a draft looks like — so this is a run created before"
            " the pull request was ready and never replaced. #153 exactly."
            " Nothing compiled this commit.",
        ]

    run = real[0]
    ran = jobs.get(run.get("id"), [])
    passed = [job for job in ran if job.get("conclusion") == "success"]
    lines = [f"{WORKFLOW} passed on {short}: {len(passed)} jobs ran."]
    skipped = [job["name"] for job in ran if job.get("conclusion") == "skipped"]
    if skipped:
        # The app gate skips by design when a branch touches nothing under
        # `app/`. Naming what did not run is the difference between this and
        # the report it was written to distrust.
        lines.append(f"Skipped, and expected to be: {', '.join(skipped)}.")
    if len(runs) > 1:
        # Said out loud rather than quietly resolved, because "which run
        # answered" is the question #245 was about.
        lines.append(
            f"{len(runs)} runs exist for this commit; none failed, and this is"
            " the one that built it."
        )
    return True, lines


def main() -> int:
    if len(sys.argv) != 2:
        sys.exit("usage: mergeable.py PULL_REQUEST_NUMBER")
    number = sys.argv[1]

    repo = gh("repo", "view", "--json", "nameWithOwner")["nameWithOwner"]
    pull = gh("pr", "view", number, "--json", "isDraft,headRefOid,number")
    sha = pull["headRefOid"]

    listed = gh("api", f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100")
    runs = runs_for(listed.get("workflow_runs", []), sha)
    # Asked of each run itself rather than of the pull request, because
    # `gh pr checks` blends jobs from every run on the branch — which is how a
    # skipped run and a green one can appear as one passing report.
    jobs = {
        run["id"]: gh(
            "api", f"repos/{repo}/actions/runs/{run['id']}/jobs"
        ).get("jobs", [])
        for run in runs
    }

    ok, lines = judge(pull, runs, jobs)
    head, *rest = lines
    where = sys.stdout if ok else sys.stderr
    print(f"mergeable: #{number}: {head}", file=where)
    for line in rest:
        print(f"  {line}", file=where)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
