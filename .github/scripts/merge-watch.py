"""What `make queue WATCH=1` takes next — the choosing half of the watch (#690).

`merge-queue.py --watch` is the same queue as `merge-queue.py N M ...`, with
one difference: instead of a list typed in by whoever started it, it asks
GitHub every poll which pull requests are cleared to merge, and takes the best
one. Everything that happens *to* a pull request once taken — rebase, push,
wait for a run to exist on the new head, `mergeable`, merge, hand back — is the
queue's [`take`], unchanged. This module only decides *which one, and whether
at all*, and every function in it is pure except [`waiting`] and
[`issue_labels`], which ask.

## Why a list was not enough

The batch of 2026-10-02/03 (27 pull requests, all cloud-coded) spent its
orchestrator on feeding the queue: a hand-rolled chain that waited on itself
through `pgrep -f`, a file-backed runner in `/tmp` that a reboot erased, and a
watcher re-armed by hand to learn when a pull request turned ready. Each of
those is the orchestrator keeping state GitHub already keeps. Here the state
*is* GitHub: a crash loses nothing, because the next watch reads the same
labels.

## The go-ahead is a label, and readiness alone is not one

A cloud coder marks its own pull request ready the moment its gates are green,
before anybody has read the diff — and `issue-batch` makes reading the diff the
orchestrator's first step, because a green gate does not read code. So *ready*
cannot be the trigger. Two signals were weighed:

- **An approving review.** Native, but this repo's pull requests are opened
  under the owner's own account, and GitHub refuses an approval from a pull
  request's author — the signal could not be given at all. It also needs the
  review API, which the agents' GitHub tools reach awkwardly.
- **A label, [`QUEUE_LABEL`].** One call to add, visible on the pull request
  and in any listing, removable to pull a branch back out, and readable in the
  same `gh pr list` that finds the candidates. It lives in `.github/labels.json`
  like every other label.

So the label. Its meaning is narrow: *a human or the orchestrator has read
this and it may merge when CI says so*. It is not a verdict on CI — the queue
still asks `mergeable` about every head it merges.

## Order: the fewest conflicts, then the label priority, then age

**Fewest conflicts first** (#738). Reordering cannot make two conflicting pull
requests merge cleanly — whichever lands second still conflicts — but it
decides *how many hand-backs* a set of them costs, and each hand-back is a
rebase session, the expensive part of a batch. On 2026-10-03 #677 touched
every registry and landed first, which handed back #676, #686 and #673, rebased
one after another at about an hour each. Landing the three small appenders
first would have cost one hand-back, on #677, resolving all three in one pass.
So at every pick the watch counts, for each pull request in line, how many of
the others it textually conflicts with ([`clashing`], `git merge-tree
--write-tree` per pair: no checkout, no build, milliseconds), and takes the one
with the fewest. A head that was handed back and has not moved is not in line,
so it does not count against anybody. Semantic breaks that merge cleanly — a
renamed function, a duplicate migration number — are invisible here and are
caught elsewhere (CI, and the queue's own numbering check).

It **fails open**: a pair `merge-tree` cannot answer about — an object that
would not fetch, an old git — counts as no conflict, so the worst case is
today's order. Ordering is an optimisation and never stops the queue. When the
count does change who goes first, the watch says so in one line
([`reordered`]), so a report shows why a lower-priority pull request went
ahead.

**Then the label priority.** `CLAUDE.md` orders what gets merged:
infrastructure → architecture → bug → foundation → feature. Infrastructure
leads because every branch behind it runs on the faster loop; a queue that took
pull requests in arrival order would merge a feature ahead of the CI fix every
later run needs. Documentation "never waits its turn", so it ranks with the
first. A pull request rarely carries the type label itself — its **issue**
does — so the labels read are the pull request's own plus those of the issues
it closes. Within a rank, the oldest pull request first: it has waited longest
and has paid the most rebases already.

## Dependabot labels itself, and goes after everything else

Dependabot's pull requests (`.github/dependabot.yml`, #721) carry the `queue`
label from the moment they open: the config adds it. They are semver-compatible
lockfile bumps with nothing in the diff for anyone to read, so the go-ahead is
CI alone, and an orchestrator labelling each by hand would add a step and no
judgement. What keeps that safe is the order: a bot pull request goes after
every other one, labelled or not ([`ordered`]), so a dependency bump never
takes a turn from the work of the batch. It is recognised by its author
([`is_bot`]) rather than by a label, because a label is something anybody can
add or remove. What the queue does differently once it has taken one — it
never force-pushes Dependabot's branch — is `merge-queue.py`'s.

## A hand-back is remembered by head

A pull request the queue handed back keeps its label, so the next poll would
find it again and retry a conflict every thirty seconds. The watch remembers
the head it handed back, and a pull request is skipped while its head is still
that commit. A push — the coder's fix, or a rebase — is a new head, and the
watch takes it again on its own. That is the whole re-queue protocol: push.
The memory is per process, so a fresh watch retries everything once.

## It fits the harness's two-hour cap

The agents that run this run it as a background command, and the harness
kills one at two hours. So the watch takes new pull requests for `--for`
minutes and then stops taking; the one in flight has its own `--deadline`;
and the two together are held under [`CAP_MINUTES`], leaving minutes for the
last rebase and merge. A watch that ends with pull requests still in line
reports each as **unfinished** — not handed back, nothing is wrong with them —
and the answer is to start the watch again.
"""

from __future__ import annotations

import importlib.util
import subprocess
from itertools import combinations
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "mergeable", Path(__file__).resolve().parent / "mergeable.py"
)
mergeable = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mergeable)

# The go-ahead. See the module doc for why a label and not a review.
QUEUE_LABEL = "queue"

# CLAUDE.md's merge priority, lower first. Documentation never waits its turn,
# so it shares the first rank. Anything else — no type label at all — goes last.
PRIORITY = {
    "infrastructure": 0,
    "documentation": 0,
    "architecture": 1,
    "bug": 2,
    "foundation": 3,
    "feature": 4,
}
UNRANKED = len(set(PRIORITY.values()))

# How `gh` names Dependabot as a pull request's author: `gh pr list/view --json
# author` says `app/dependabot`; the REST API and the web say `dependabot[bot]`.
BOTS = ("app/dependabot", "dependabot[bot]", "dependabot")

# How long a watch takes new pull requests, in minutes, by default. With the
# queue's per-branch deadline of 40 that is 110, inside the cap below.
FOR_MINUTES = 70

# The harness stops a background command at 120 minutes. `--for` plus
# `--deadline` may not exceed this, which leaves five for the last branch's
# rebase, push and merge on either side of its wait.
CAP_MINUTES = 115

# How often an idle watch asks GitHub again. Slower than a CI poll: nothing is
# being waited on, and a pull request labelled a minute late costs a minute.
IDLE_SECONDS = 60


def rank(labels: list[str]) -> int:
    """The best priority among these labels, or [`UNRANKED`]."""
    return min((PRIORITY[name] for name in labels if name in PRIORITY), default=UNRANKED)


def is_bot(pull: dict) -> bool:
    """Whether Dependabot opened this pull request, and so owns its branch."""
    return (pull.get("author") or {}).get("login") in BOTS


def eligible(pull: dict, dropped: dict[int, str]) -> bool:
    """Whether the watch may take this pull request now.

    Ready (a draft asserts nothing, and the queue would only hand it back),
    against `main`, and not still sitting on the head the watch already handed
    back or finished with.
    """
    return (
        not pull.get("isDraft")
        and pull.get("baseRefName", "main") == "main"
        and dropped.get(pull.get("number")) != pull.get("headRefOid")
    )


def labels_of(pull: dict, issues: dict[int, list[str]]) -> list[str]:
    """The pull request's own labels plus those of the issues it closes.

    `issues` maps an issue number to its label names; one missing from it is
    simply not counted.
    """
    own = [label.get("name", "") for label in pull.get("labels", [])]
    closed = [
        name
        for ref in pull.get("closingIssuesReferences", []) or []
        for name in issues.get(ref.get("number"), [])
    ]
    return own + closed


def contenders(pulls: list[dict], dropped: dict[int, str]) -> list[dict]:
    """The pull requests whose conflicts with one another decide the order.

    Dependabot is left out: it goes last whatever it conflicts with, and it
    rebases its own branch for free, so its conflicts cost nobody a session.
    """
    return [pull for pull in pulls if eligible(pull, dropped) and not is_bot(pull)]


def conflict_counts(
    numbers: list[int], clashes: set[frozenset[int]]
) -> dict[int, int]:
    """How many of the others in `numbers` each one conflicts with.

    `clashes` holds the conflicting pairs; a pair naming somebody no longer in
    line does not count.
    """
    present = set(numbers)
    return {
        number: sum(1 for pair in clashes if number in pair and pair <= present)
        for number in numbers
    }


def ordered(
    pulls: list[dict],
    issues: dict[int, list[str]],
    clashes: set[frozenset[int]] = frozenset(),
) -> list[dict]:
    """Dependabot last, then fewest conflicts, then highest priority, then oldest.

    `clashes` holds the pairs of pull requests that conflict with each other,
    counted only among those in `pulls`. `createdAt` is ISO 8601, so it sorts.
    """
    counts = conflict_counts([pull.get("number") for pull in pulls], clashes)
    return sorted(
        pulls,
        key=lambda pull: (
            is_bot(pull),
            counts.get(pull.get("number"), 0),
            rank(labels_of(pull, issues)),
            pull.get("createdAt", ""),
            pull.get("number", 0),
        ),
    )


def pick(
    pulls: list[dict],
    dropped: dict[int, str],
    issues: dict[int, list[str]],
    clashes: set[frozenset[int]] = frozenset(),
) -> dict | None:
    """The pull request to take next, or `None` when none may be taken."""
    ready = [pull for pull in pulls if eligible(pull, dropped)]
    return ordered(ready, issues, clashes)[0] if ready else None


def reordered(
    pulls: list[dict],
    dropped: dict[int, str],
    issues: dict[int, list[str]],
    clashes: set[frozenset[int]],
) -> str | None:
    """One line when the conflict count put somebody ahead of label order, else `None`."""
    chosen = pick(pulls, dropped, issues, clashes)
    by_label = pick(pulls, dropped, issues)
    if chosen is None or by_label is None or chosen is by_label:
        return None
    ready = [pull["number"] for pull in pulls if eligible(pull, dropped)]
    counts = conflict_counts(ready, clashes)
    first, passed = chosen["number"], by_label["number"]
    return (
        f"#{first} goes before #{passed}, which labels alone would take: #{first}"
        f" conflicts with {counts[first]} other(s) in line, #{passed} with"
        f" {counts[passed]}."
    )


def _git(root: str, *args: str) -> tuple[int, str]:
    """`git`'s exit status and output, or `(-1, "")` when it could not run at all."""
    try:
        done = subprocess.run(
            ["git", *args], capture_output=True, text=True, check=False, cwd=root
        )
    except OSError:
        return -1, ""
    return done.returncode, done.stdout


def conflicted(status: int, output: str) -> bool:
    """Whether `git merge-tree --write-tree` reported a conflict.

    Exit 1 is not enough: git also exits 1 for a head it cannot find ("not
    something we can merge"). A real conflict prints the merged tree's id
    first, so that is asked for too; anything else fails open.
    """
    first = output.split("\n", 1)[0].strip()
    return status == 1 and len(first) in (40, 64) and all(c in "0123456789abcdef" for c in first)


def clashing(
    pulls: list[dict], root: str, known: dict[frozenset[str], bool]
) -> set[frozenset[int]]:
    """The pairs of these pull requests whose heads textually conflict.

    `git merge-tree --write-tree` per pair, read by [`conflicted`]: anything
    but a reported conflict counts as clean (fail open — see the module doc). `known` remembers each pair of *heads*, so a poll that sees the same
    heads asks git nothing. Heads this checkout lacks are fetched by sha, once,
    together; a failed fetch is only more pairs that fail open.
    """
    heads = {pull["number"]: pull.get("headRefOid", "") for pull in pulls}
    pairs = [
        (a, b) for a, b in combinations(sorted(heads), 2) if heads[a] and heads[b]
    ]
    unknown = [pair for pair in pairs if frozenset((heads[pair[0]], heads[pair[1]])) not in known]
    if unknown:
        wanted = {heads[n] for pair in unknown for n in pair}
        missing = [
            sha for sha in sorted(wanted)
            if _git(root, "cat-file", "-e", f"{sha}^{{commit}}")[0] != 0
        ]
        if missing:
            _git(root, "fetch", "--quiet", "origin", *missing)
        for a, b in unknown:
            merged = _git(root, "merge-tree", "--write-tree", heads[a], heads[b])
            known[frozenset((heads[a], heads[b]))] = conflicted(*merged)
    return {
        frozenset(pair)
        for pair in pairs
        if known.get(frozenset((heads[pair[0]], heads[pair[1]])), False)
    }


def in_line(pulls: list[dict], dropped: dict[int, str]) -> list[int]:
    """What the watch would still have taken — reported as unfinished at its end."""
    return sorted(pull["number"] for pull in pulls if eligible(pull, dropped))


def budget_error(watch_for: float, deadline: float) -> str | None:
    """Why these limits do not fit the harness's cap, or `None` if they do."""
    if watch_for <= 0:
        return "--for must be more than zero minutes."
    if watch_for + deadline > CAP_MINUTES:
        return (
            f"--for {watch_for:g} plus --deadline {deadline:g} is"
            f" {watch_for + deadline:g} minutes; a watch must fit in"
            f" {CAP_MINUTES}, because the harness kills a background command at"
            " two hours. Shorten one, and start another watch when it ends."
        )
    return None


def waiting(label: str = QUEUE_LABEL) -> list[dict]:
    """Every open pull request carrying the go-ahead, in the fields the watch reads."""
    return mergeable.gh(
        "pr",
        "list",
        "--state",
        "open",
        "--label",
        label,
        "--limit",
        "100",
        "--json",
        "number,isDraft,headRefOid,baseRefName,createdAt,author,labels,closingIssuesReferences",
    )


def issue_labels(pulls: list[dict], known: dict[int, list[str]]) -> dict[int, list[str]]:
    """Fill `known` with the labels of every issue these pull requests close.

    Asked once per issue per watch: an issue's type label does not change
    while its pull request waits, and one call per candidate per poll would be
    most of the watch's traffic.
    """
    for pull in pulls:
        for ref in pull.get("closingIssuesReferences", []) or []:
            number = ref.get("number")
            if number is None or number in known:
                continue
            found = mergeable.gh("issue", "view", str(number), "--json", "labels")
            known[number] = [label.get("name", "") for label in found.get("labels", [])]
    return known
