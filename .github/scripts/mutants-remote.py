#!/usr/bin/env python3
"""Ask GitHub's runners for a mutation run over exactly SCOPE, wait, and print it.

`make mutants-remote SCOPE=…` is this. It dispatches `mutants-on-request.yml`
on the current branch, finds the run it started, waits for it, downloads the
`mutants-report` artifact and prints the report and the survivors' diffs — so
an agent reads the answer in the terminal, not by scrolling a log (#651).

Why remote at all, when `make mutants` exists: mutation compiles every mutant
from scratch, two at a time, for as long as the scope takes. On the
operator's machine that competes with sibling worktrees' builds and with the
web app's users (CLAUDE.md, *How we work*); on a runner it costs nothing local.
`make mutants` stays for whoever wants it here.

Three rules, each one a way the old per-pull-request signal or `make
mergeable` learned to fail:

- **The run must exist before anything is said about it.** A dispatch returns
  no run id, so the run is found by the request id this generates, which the
  workflow puts in its run name. Not finding it is an error with that said —
  never a silence that could be read as a clean result (`ci-merge`, on absent
  and passing being different claims).
- **The runner mutates what GitHub has, not what is here.** A branch whose
  local head is not the pushed head is refused before dispatch, because the
  report would describe code this checkout does not hold.
- **No report is never zero survivors.** A run that ends red, cancelled or
  timed out, or green without its artifact, exits 1 and says which. Survivors
  are a result: they exit 0, like `make mutants`.

Questions to GitHub go through `mergeable.gh`, which retries a failure in
transit and raises `Unreachable` (exit 3) when the network outlasts it. The
dispatch does not: an action retried blindly can happen twice.
"""

from __future__ import annotations

import argparse
import importlib.util
import secrets
import subprocess
import sys
import time
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "mergeable", Path(__file__).resolve().parent / "mergeable.py"
)
assert _spec and _spec.loader
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)

WORKFLOW = "mutants-on-request.yml"

#: How long a dispatched run may take to appear in the run list. GitHub
#: usually lists it within seconds; five minutes is a queue under load.
FIND_SECONDS = 300

#: The longest wait for a run to finish: a plan, a shard's two-hour limit and
#: the report, plus queueing behind every ready pull request's CI.
WAIT_SECONDS = 4 * 3600

#: Past this many lines the survivors' diffs are left in the file and named,
#: rather than scrolled past.
PRINT_DIFF_LINES = 400


def say(message: str) -> None:
    print(f"mutants-remote: {message}", file=sys.stderr, flush=True)


def git(*args: str) -> str:
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout.strip()


def unpushed(branch: str, local: str, remote: str | None) -> str | None:
    """Why the runner would not be mutating this checkout, or `None` if it would."""
    if branch == "HEAD":
        return "HEAD is detached; dispatch needs a branch GitHub has"
    if remote is None:
        return f"origin/{branch} does not exist; push the branch first"
    if remote != local:
        return (
            f"origin/{branch} is {remote[:9]} and HEAD is {local[:9]}; the runner"
            " mutates what GitHub has, so push (or pull) first"
        )
    return None


def find(runs: list[dict], request: str) -> dict | None:
    """The run this request started, recognised by the id in its name."""
    tag = f"[{request}]"
    return next((r for r in runs if tag in r.get("displayTitle", "")), None)


def verdict(run: dict, have_report: bool) -> tuple[bool, str]:
    """Whether there is a report to print, and the sentence that says so."""
    conclusion = run.get("conclusion") or "unfinished"
    url = run.get("url", "")
    if conclusion != "success":
        return False, (
            f"the run ended {conclusion}, with no report to read: {url}. That is"
            " not a clean result; nothing about the code is known from it."
        )
    if not have_report:
        return False, f"the run passed but left no mutants-report artifact: {url}"
    return True, f"report from {url}"


def dispatch(branch: str, scope: str, request: str) -> None:
    done = subprocess.run(
        ["gh", "workflow", "run", WORKFLOW, "--ref", branch,
         "-f", f"scope={scope}", "-f", f"request={request}"],
        capture_output=True, text=True, check=False,
    )
    if done.returncode != 0:
        sys.exit(f"mutants-remote: dispatch refused: {done.stderr.strip()}")


def located(branch: str, request: str) -> dict:
    deadline = time.monotonic() + FIND_SECONDS
    while True:
        runs = mergeable.gh(
            "run", "list", "--workflow", WORKFLOW, "--branch", branch,
            "--event", "workflow_dispatch", "--limit", "30",
            "--json", "databaseId,displayTitle,headSha,url,status,conclusion",
        )
        run = find(runs, request)
        if run is not None:
            return run
        if time.monotonic() > deadline:
            sys.exit(
                f"mutants-remote: dispatched request {request}, but no run carrying"
                f" it appeared within {FIND_SECONDS}s. Nothing ran that this knows of."
            )
        time.sleep(10)


def finished(run_id: int, poll: int) -> dict:
    deadline, last = time.monotonic() + WAIT_SECONDS, None
    while True:
        run = mergeable.gh("run", "view", str(run_id), "--json", "status,conclusion,url")
        if run["status"] != last:
            say(f"run {run_id} is {run['status']}")
            last = run["status"]
        if run["status"] == "completed":
            return run
        if time.monotonic() > deadline:
            sys.exit(f"mutants-remote: still {run['status']} after {WAIT_SECONDS}s: {run['url']}")
        time.sleep(poll)


def download(run_id: int, out: Path) -> bool:
    done = subprocess.run(
        ["gh", "run", "download", str(run_id), "-n", "mutants-report", "-D", str(out)],
        capture_output=True, text=True, check=False,
    )
    return done.returncode == 0 and (out / "report.md").is_file()


def show(out: Path) -> None:
    print((out / "report.md").read_text())
    diffs = out / "survivors.diff"
    text = diffs.read_text() if diffs.is_file() else ""
    if not text.strip():
        return
    lines = text.splitlines()
    if len(lines) > PRINT_DIFF_LINES:
        print(f"\nThe survivors' diffs are {len(lines)} lines: {diffs}")
        return
    print("\n## The survivors' diffs\n")
    print(text)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="mutants-remote.py", description=__doc__.split("\n")[0])
    parser.add_argument("scope", help="`diff`, a package (scorsese-core), or path globs")
    parser.add_argument("--out", type=Path, default=Path("target/mutants-remote"))
    parser.add_argument("--poll", type=int, default=30, help="seconds between status checks")
    args = parser.parse_args(argv)

    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    subprocess.run(["git", "fetch", "--quiet", "origin", branch], check=False)
    remote = subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", f"origin/{branch}"],
        capture_output=True, text=True, check=False,
    ).stdout.strip() or None
    refusal = unpushed(branch, git("rev-parse", "HEAD"), remote)
    if refusal:
        sys.exit(f"mutants-remote: {refusal}")

    request = secrets.token_hex(4)
    dispatch(branch, args.scope, request)
    say(f"dispatched {args.scope!r} on {branch} as request {request}")
    try:
        run = located(branch, request)
        say(f"run {run['databaseId']}: {run['url']}")
        run = finished(run["databaseId"], args.poll) | {"databaseId": run["databaseId"]}
    except mergeable.Unreachable as unreachable:
        say(str(unreachable))
        return mergeable.UNREACHABLE_STATUS
    out = args.out / str(run["databaseId"])
    have = run.get("conclusion") == "success" and download(run["databaseId"], out)
    ok, sentence = verdict(run, have)
    say(sentence)
    if not ok:
        return 1
    show(out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
