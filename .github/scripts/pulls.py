"""Pull requests over REST, in the shape the merge path reads (#1724).

`make mergeable`, `make queue` and `make described` ask GitHub through `gh
api` alone: a cloud container's proxy refuses GraphQL, which `gh`'s pull
request, issue and repository commands are underneath. This module owns the
mapping from REST's answers onto the GraphQL field names (`isDraft`,
`headRefOid`, …) the pure functions read, so none of them knows which API
answered, and it builds the three writes (merge, comment, re-run) the queue
sends.

REST has no `closingIssuesReferences`: it is the body's closing keywords
([`closing.in_body`]), so an issue linked only in the sidebar is not seen. A
listing carries no mergeability: it reads `UNKNOWN` there, and only a single
pull request's record ([`view`]) says `MERGEABLE` or `CONFLICTING`.

Pure but for [`view`] and [`open_pulls`], which ask through the `gh` they are
handed (`mergeable.gh`), and [`repository`], which asks git.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import closing  # noqa: E402

# Owner and name at the end of any spelling of a GitHub remote: https, ssh, or
# the cloud container's proxy (`…/git/<owner>/<name>`).
REMOTE = re.compile(r"[/:]([^/:]+)/([^/]+?)(?:\.git)?/?$")

# A page of a REST listing; a shorter page is the last.
PAGE = 100

# REST's `mergeable` (a boolean, or null while GitHub computes it) as the word
# GraphQL says.
MERGEABLE = {True: "MERGEABLE", False: "CONFLICTING"}


def from_rest(rest: dict) -> dict:
    """`repos/{repo}/pulls/N`, or one entry of the listing, in GraphQL's field names."""
    head, base = rest.get("head") or {}, rest.get("base") or {}
    body = rest.get("body") or ""
    repo = (base.get("repo") or {}).get("full_name", "")
    state = "MERGED" if rest.get("merged") else str(rest.get("state") or "").upper()
    return {
        "number": rest.get("number"),
        "state": state,
        "isDraft": bool(rest.get("draft")),
        "headRefOid": head.get("sha", ""),
        "headRefName": head.get("ref", ""),
        "baseRefName": base.get("ref", ""),
        "body": body,
        "author": {"login": (rest.get("user") or {}).get("login", "")},
        "labels": [{"name": label.get("name", "")} for label in rest.get("labels") or []],
        "createdAt": rest.get("created_at", ""),
        "mergedAt": rest.get("merged_at"),
        "mergeable": MERGEABLE.get(rest.get("mergeable"), "UNKNOWN"),
        "mergeStateStatus": str(rest.get("mergeable_state") or "unknown").upper(),
        "closingIssuesReferences": [{"number": n} for n in closing.in_body(body, repo)],
    }


def view(gh, repo: str, number: int | str) -> dict:
    """One pull request as GitHub has it now."""
    return from_rest(gh("api", f"repos/{repo}/pulls/{number}"))


def open_pulls(gh, repo: str) -> list[dict]:
    """Every open pull request, page by page."""
    found, page = [], 1
    while True:
        batch = gh("api", f"repos/{repo}/pulls?state=open&per_page={PAGE}&page={page}")
        found += [from_rest(rest) for rest in batch]
        if len(batch) < PAGE:
            return found
        page += 1


def labelled(pulls: list[dict], label: str) -> list[dict]:
    """The pull requests carrying `label`: REST's listing cannot filter by one."""
    return [pull for pull in pulls if label in closing.labels(pull)]


def owner_and_name(url: str) -> str:
    """`owner/name` from a git remote's URL."""
    match = REMOTE.search(url.strip())
    if not match:
        sys.exit(f"pulls: cannot read a GitHub repository from the remote {url!r}.")
    return f"{match.group(1)}/{match.group(2)}"


def repository(root: str | None = None) -> str:
    """This checkout's repository, from `origin`: no GraphQL call."""
    url = subprocess.run(["git", "remote", "get-url", "origin"], capture_output=True,
                         text=True, check=True, cwd=root).stdout
    return owner_and_name(url)


def merge_call(repo: str, number: int, sha: str) -> list[str]:
    """The squash merge, refused by GitHub (409) if the head is no longer `sha`."""
    return ["gh", "api", "-X", "PUT", f"repos/{repo}/pulls/{number}/merge",
            "-f", "merge_method=squash", "-f", f"sha={sha}"]


def comment_call(repo: str, number: int, body: str) -> list[str]:
    """A comment on the pull request's conversation."""
    return ["gh", "api", "-X", "POST", f"repos/{repo}/issues/{number}/comments",
            "-f", f"body={body}"]


def rerun_call(repo: str, run_id: int) -> list[str]:
    """Re-run a workflow run's failed jobs."""
    return ["gh", "api", "-X", "POST", f"repos/{repo}/actions/runs/{run_id}/rerun-failed-jobs"]
