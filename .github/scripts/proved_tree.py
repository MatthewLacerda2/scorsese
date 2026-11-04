#!/usr/bin/env python3
"""`proved_tree.py` — was this push to `main` already proved by a run of its tree? (#1758)

Every change reaches `main` through `make queue`, which squash-merges a head
whose CI run judged the exact tree that lands. Re-running CI on `main` tested
that tree a second time: 26 runs in the 2026-11-03 batch, none finding anything
real, two going red on a flake nobody saw, each holding runners the queue's next
pull request waited on. So `ci.yml`'s `changes` job asks this script on a push
to `main`, and every job that builds is skipped when the answer is yes. Ported
from rusty's (its #1784/#1785).

**Mechanical, not assumed.** The pushed commit's tree is compared with the head
tree of `ci.yml`'s recent successful runs on `main`, of pull requests into
`main` (a stacked pull request's run tested another base), or dispatched by the
merge queue on a train's branch ([`TRAIN_PREFIX`], #1765): a train's run built
the tree its last member's squash leaves. A squash merge of an up-to-date head
has exactly the head's tree, so a match proves the case rather than trusting
that the queue did it: a merge from GitHub's page whose head no run built, or a
push nothing ran on, runs in full. A run counts only when it really built: each
job in [`BUILT`] concluded `success` and its named step ran and succeeded, so a
draft's run (every job skipped) and a Markdown-only run (the heavy jobs skipped)
prove nothing. A `pull_request` run tests the merge of its head into the base of
that moment, not the head; when the squash's tree equals the head's, every
change on `main` is already in the head, so that merge was the head's tree too.

**Caches keep their full run.** Each cache `ci.yml` saves from `main` is keyed
on files: `Swatinem/rust-cache` on the toolchain, every `Cargo.toml` /
`Cargo.lock` and cargo's `.cargo/config.toml` (not `.cargo/mutants.toml`), the
apt archives and the Chromium download on the scripts that fetch them, and every
one of them on the job's env, which lives in the workflow. A push that changes
any `CACHE_KEYED_*` path since the previous tip, or the workflow itself, runs
everything, so the new key's cache is written by `main` and a dependency bump is
proven on `main`. Any other push leaves the keys, and so the saved caches, as
they were. What moves with no file (the runner image the apt key names) is the
nightly's: `ci.yml`'s `schedule` builds `main` in full and saves.

**A superseded push skips.** A merge train lands its members as one squash
each, seconds apart (#1757), so every push but the last carries a tree no run
built and `main` leaves at once. When the pushed commit is no longer `main`'s
tip (`repos/{repo}/commits/main`) and code landed after it, its build is
skipped: the newer tip's own push run covers it. Only after code: a push run
whose diff is Markdown-only asks nothing and builds nothing, so it covers no
earlier tree. That is sound: `main` is linear, each later tip holds every
change of the one before, nothing ships the intermediate tree, and the train's
own run proved the tree its last squash leaves. A run that starts before the
next merge sees itself as the tip and simply builds. The cache-key check comes
first, so a superseded push that moves a key still runs in full and saves.

**Fail toward running.** Any error (an API failure, a field missing, a compare
too long to read) answers no, and the push runs every gate.

Writes `proved=true|false` to `$GITHUB_OUTPUT` when set, prints the reason
(with the proving run's url), and exits 0 on every answer it reaches. What it
cannot catch (no `gh`, a hung call) the workflow step absorbs: it runs with
`continue-on-error` and a short timeout, and an empty output means a full run.
REST only (`test_rest_only.py`). Env: `GH_TOKEN`, `GITHUB_REPOSITORY`.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Callable

_spec = importlib.util.spec_from_file_location("ci_paths", Path(__file__).with_name("ci-paths.py"))
ci_paths = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ci_paths)

sys.path.insert(0, str(Path(__file__).resolve().parent))
from merge_queue.train import PREFIX as TRAIN_PREFIX  # noqa: E402  (the queue's own name for a train's branch)

WORKFLOW = "ci.yml"

#: The jobs a run must have built in, by the `name:` the jobs API reports, each
#: with the step that does the checking. `regression test` is left out: on a
#: pull request that owes no test its steps skip, and on `main` it owes none.
BUILT: dict[str, str] = {
    "fmt + clippy + test": "Test",
    "desktop app": "Test",
    "web front-end": "Test",
    "size gate": "Run the gate",
    "signal renderers": "Test",
    "deploy config": "Terraform",
    "cargo-deny": "Both workspaces",
}

#: What a cache key hashes: by file name anywhere, by exact path, by cargo config.
CACHE_KEYED_NAMES = ("Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml")
CACHE_KEYED_CONFIGS = (".cargo/config", ".cargo/config.toml")
CACHE_KEYED_PATHS = ("tools/ci-apt", "tools/ci-postgres", "tools/chromium/pin",
                     f".github/workflows/{WORKFLOW}")

#: How many pages of successful runs to search; the proving run of a merge is
#: the pull request's last, or its train's, minutes old.
PAGES = 2
PER_PAGE = 100
#: The compare API lists at most this many files; a longer list is unread.
COMPARE_CAP = 300
NO_COMMIT = "0" * 40

Get = Callable[[str], object]


def gh_get(path: str):
    """`gh api <path>`, parsed; `None` on any failure."""
    done = subprocess.run(["gh", "api", path], capture_output=True, text=True, check=False)
    if done.returncode != 0:
        return None
    try:
        return json.loads(done.stdout)
    except json.JSONDecodeError:
        return None


def keyed(path: str) -> bool:
    """Whether a changed path moves a cache key `main` saves under."""
    name = path.rsplit("/", 1)[-1]
    return (name in CACHE_KEYED_NAMES or path in CACHE_KEYED_PATHS
            or any(path == c or path.endswith("/" + c) for c in CACHE_KEYED_CONFIGS))


def changed(get: Get, repo: str, base: str, head: str) -> list[str] | None:
    """Every path `base...head` touches, both ends of a rename, when `head` is
    a plain fast-forward of `base` and the list is whole; `None` otherwise."""
    compare = get(f"repos/{repo}/compare/{base}...{head}")
    if not isinstance(compare, dict) or compare.get("status") != "ahead":
        return None
    files = compare.get("files")
    if not isinstance(files, list) or len(files) >= COMPARE_CAP:
        return None
    return [name for f in files for name in (f["filename"], f.get("previous_filename")) if name]


def key_change(get: Get, repo: str, before: str, sha: str) -> str | None:
    """Why this push must run in full for its caches; `None` when it need not."""
    if not before or before == NO_COMMIT:
        return "no previous tip to compare with"
    files = changed(get, repo, before, sha)
    if files is None:
        return "the push is not a plain fast-forward of the previous tip, or its files could not all be read"
    hit = sorted({f for f in files if keyed(f)})
    return f"it changes {', '.join(hit)}" if hit else None


def built(jobs: list[dict]) -> bool:
    """Whether every job [`BUILT`] names concluded `success` with its step run."""
    for name, step in BUILT.items():
        mine = [j for j in jobs if j.get("name") == name]
        if not mine or any(j.get("conclusion") != "success" for j in mine):
            return False
        if not all(any(s.get("name") == step and s.get("conclusion") == "success"
                       for s in j.get("steps") or []) for j in mine):
            return False
    return True


def against_main(run: dict) -> bool:
    """Whether the run tested its tree as `main` holds it: a run on `main`
    itself, a pull request's whose base is `main`, or the merge queue's
    dispatch of a train, whose tip is `main` with the members squashed on. A
    stacked pull request's run tested its head merged into another branch, and
    no other dispatch is the queue's."""
    if run.get("event") in ("push", "schedule"):
        return run.get("head_branch") == "main"
    if run.get("event") == "workflow_dispatch":
        return str(run.get("head_branch") or "").startswith(TRAIN_PREFIX)
    if run.get("event") != "pull_request":
        return False
    return any(((pr.get("base") or {}).get("ref")) == "main" for pr in run.get("pull_requests") or [])


def proving_run(get: Get, repo: str, tree: str) -> str | None:
    """The url of a successful run that built `tree`, if one is recent."""
    for page in range(1, PAGES + 1):
        answer = get(f"repos/{repo}/actions/workflows/{WORKFLOW}/runs?status=success"
                     f"&per_page={PER_PAGE}&page={page}")
        if not isinstance(answer, dict):
            raise LookupError("the runs list did not come")
        runs = answer["workflow_runs"]
        for run in runs:
            if (run.get("head_commit") or {}).get("tree_id") != tree or not against_main(run):
                continue
            jobs = get(f"repos/{repo}/actions/runs/{run['id']}/jobs?filter=latest&per_page=100")
            if not isinstance(jobs, dict):
                raise LookupError(f"the jobs of run {run['id']} did not come")
            if built(jobs["jobs"]):
                return run["html_url"]
        if len(runs) < PER_PAGE:
            break
    return None


def superseded(get: Get, repo: str, sha: str) -> str | None:
    """`main`'s tip when it is now a commit other than `sha` whose push run is
    asked too: something between them is code, so the tip's `changes` does not
    stop at Markdown-only. Any doubt is `None`."""
    tip = get(f"repos/{repo}/commits/main")
    found = tip.get("sha") if isinstance(tip, dict) else None
    if not (isinstance(found, str) and len(found) == 40 and found != sha):
        return None
    files = changed(get, repo, sha, found)
    return found if files and ci_paths.runs_for(ci_paths.PATTERNS, files) else None


def verdict(get: Get, repo: str, sha: str, before: str) -> tuple[str | None, str]:
    """(what proves it: the proving run's url, or the newer tip that supersedes
    it; `None` when nothing does) and why, failing toward running on any error."""
    try:
        why = key_change(get, repo, before, sha)
        if why:
            return None, f"runs in full: {why}"
        newer = superseded(get, repo, sha)
        if newer:
            return newer, f"skips the build: `main` is at {newer} now, past {sha}, and that tip's own run covers it"
        commit = get(f"repos/{repo}/commits/{sha}")
        if not isinstance(commit, dict):
            return None, "runs in full: the pushed commit could not be read"
        tree = commit["commit"]["tree"]["sha"]
        url = proving_run(get, repo, tree)
    except (KeyError, TypeError, LookupError) as error:
        return None, f"runs in full: the lookup failed ({error!r})"
    if url is None:
        return None, f"runs in full: no recent successful {WORKFLOW} run built tree {tree}"
    return url, f"skips the build: tree {tree} was proved by {url}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--sha", required=True)
    parser.add_argument("--before", default="")
    args = parser.parse_args()
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    url, why = verdict(gh_get, repo, args.sha, args.before) if repo else (
        None, "runs in full: GITHUB_REPOSITORY is not set")
    print(f"{WORKFLOW} on {args.sha} {why}")
    out = os.environ.get("GITHUB_OUTPUT")
    if out:
        with open(out, "a", encoding="utf-8") as sink:
            sink.write(f"proved={'true' if url else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
