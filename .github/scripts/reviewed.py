"""Whether a pull request that needs an independent review has had one (#1498).

The `independent-review` skill sends certain pull requests to a fresh,
read-only reviewer before they merge: the ones whose kind of change is where
this repository's bugs have come from — money, stored data, web isolation,
concurrency, the render pipeline, and the gates themselves. Which kind a pull
request is can be read off the paths it changes, so the rule is a gate rather
than a sentence (`CLAUDE.md`, *A convention a machine can check is a gate*).

`make mergeable` and `make queue` hand back a pull request that changes a path
in [`TRIGGERS`] unless one of its comments starts with **Independent review**
and names a level at least the trigger's:

    ## Independent review — high

or its description carries an exception with its reason, on a line of its own:

    No independent review: only a doc comment in prices/ changed.

Markdown and test files never trigger, and neither does a pull request
Dependabot opened. No label exempts one: a `minor` pull request (a Sonnet
branch, front-end or the desktop app, which the record shows needing review
least) touches none of these paths, and one that does is not only front-end.

**A machine can check that a review was recorded, not that it was any good**:
the skill holds how it is dispatched without the coder's bias.

Pure but for [`evidence`], which asks GitHub over REST.
"""

from __future__ import annotations

import fnmatch
import re

# `code-review`'s effort levels, lowest first.
LEVELS = ("low", "medium", "high", "xhigh", "max")

# What each kind of change is, the level a miss there costs, and the paths that
# are it. A `*` crosses folders. `independent-review`'s SKILL.md, *When*, is
# the same table in prose, with the bugs each line comes from.
TRIGGERS = (
    ("money", "high", (
        "crates/providers/src/prices/*", "crates/providers/src/quote/*",
        "crates/providers/src/spending/*", "crates/providers/src/payments/*",
        "crates/providers/src/generated/*", "crates/providers/src/credentials/budget.rs",
        "crates/server/src/credits/*", "crates/server/src/payments/*",
        "crates/server/src/generations/*", "crates/server/src/assistant/cost.rs",
        "crates/server/src/assistant/quote.rs", "crates/server/src/http/credits.rs",
        "crates/server/src/http/payments.rs", "crates/cli/src/commands/generate/*",
        "crates/cli/src/commands/confirm.rs", "crates/mcp/src/tools/generate/*",
        "crates/mcp/src/tools/confirm.rs", "crates/server/src/tools/generate/*",
        "crates/server/src/tools/quotes.rs", "crates/providers/src/api/stripe/*",
        "crates/providers/src/fx/*", "app/src/generating/*", "deploy/terraform/stripe/*",
    )),
    ("stored data", "high", (
        "crates/core/src/migrate/*", "crates/core/src/write.rs",
        "crates/server/migrations/*", "crates/server/src/library/*",
        "crates/server/src/storage.rs", "crates/cli/src/commands/assets.rs",
        "crates/cli/src/commands/import.rs", "crates/cli/src/commands/migrate.rs",
        "deploy/backup/*",
    )),
    ("security", "high", (
        "crates/server/src/accounts/*", "crates/server/src/db/*",
        "crates/server/src/http/auth.rs",
        "crates/server/src/http/tokens.rs", "crates/server/src/http/google/*",
        "crates/server/src/designs/owners.rs", "crates/server/src/captures/*",
        "crates/providers/src/credentials/*", "crates/providers/src/sign_in/*",
        "deploy/capture/*", "deploy/nginx.conf", "deploy/compose*.yaml",
        "deploy/*.Dockerfile", "deploy/terraform/cloudflare/*",
        "deploy/terraform/google/*", "deploy/terraform/github/*",
    )),
    ("concurrency", "medium", (
        # The assistant's turn loop, not what it is told: its prompt and how it
        # words things are taste, and taste is not what a reviewer catches.
        "crates/server/src/jobs/*", "crates/server/src/assistant/turn.rs",
        "crates/server/src/assistant/wake.rs", "crates/server/src/assistant/stopping.rs",
        "crates/server/src/assistant/relay.rs", "crates/server/src/assistant/compact.rs",
        "crates/server/src/assistant/start.rs", "crates/server/src/assistant/calls.rs",
        "crates/server/src/assistant/quote.rs", "crates/server/src/assistant/store/*",
        "crates/server/src/assistant/ask/*",
        "crates/server/src/events.rs", "crates/server/src/http/chat.rs",
        "crates/server/src/http/events.rs", "crates/server/src/designs/job.rs",
        "crates/mcp/src/rpc.rs", "crates/mcp/src/session/*",
        "crates/mcp/src/renders/*", "crates/render/src/cancel.rs",
        "crates/render/src/workers.rs", "crates/providers/src/video/*",
    )),
    ("render", "medium", (
        "crates/render/src/run/*", "crates/render/src/pipe/*",
        "crates/render/src/frames/*", "crates/render/src/plan/*",
        "crates/render/src/page/*", "crates/render/src/audio/*",
        "crates/render/src/preview/*", "crates/server/src/renders/*",
        "crates/core/src/time/*", "crates/core/src/timeline/*",
    )),
    ("the gates", "medium", (
        ".github/workflows/*", ".github/scripts/*", "Makefile", "tools/lint/*",
        "tools/with-chrome", "tools/with-postgres", "tools/ci-postgres",
        "crates/golden/src/*",
        ".githooks/*", ".config/*", ".cargo/*", "clippy.toml", "deny.toml",
    )),
)

BOTS = ("app/dependabot", "dependabot[bot]", "dependabot")

TEST = re.compile(r"(^|/)tests?(/|\.rs$)|(^|/)test_[^/]*\.py$|\.test\.tsx?$")
HEADING = re.compile(r"^\s*#*\s*\**\s*independent review\b(.*)$", re.IGNORECASE)
EXCEPTION = re.compile(r"^\s*[-*]?\s*\**no independent review\**\s*:(.*)$",
                       re.IGNORECASE | re.MULTILINE)
WORD = re.compile(r"[A-Za-z0-9]{2,}")

# How many words an exception's reason keeps, as `reblessed.REASON_WORDS`.
REASON_WORDS = 3


def counts(path: str) -> bool:
    """Whether a changed path can trigger at all: not Markdown, not a test."""
    return not path.endswith(".md") and not TEST.search(path)


def paths_of(files: list[dict]) -> list[str]:
    """Every path these changed files touch: a rename's old path as well, so
    moving a file out of a trigger folder still counts as changing it."""
    paths = []
    for item in files:
        paths += [item.get("filename", ""), item.get("previous_filename") or ""]
    return [path for path in paths if path]


def hit(paths: list[str]) -> list[tuple[str, str, str]]:
    """Every `(area, level, path)` these paths trigger, in [`TRIGGERS`] order."""
    found = []
    for area, level, patterns in TRIGGERS:
        for path in paths:
            if counts(path) and any(fnmatch.fnmatchcase(path, p) for p in patterns):
                found.append((area, level, path))
    return found


def needed(found: list[tuple[str, str, str]]) -> str | None:
    """The level the review must reach: the highest any trigger asks for."""
    return max((level for _, level, _ in found), key=LEVELS.index, default=None)


def recorded(comments: list[str]) -> str | None:
    """The highest level an `Independent review` comment names, if any does."""
    best = None
    for body in comments:
        lines = [line for line in (body or "").splitlines() if line.strip()]
        match = HEADING.match(lines[0]) if lines else None
        if not match:
            continue
        named = [w for w in re.findall(r"[a-z]+", match.group(1).lower()) if w in LEVELS]
        for level in named:
            if best is None or LEVELS.index(level) > LEVELS.index(best):
                best = level
    return best


def excused(body: str) -> bool:
    """Whether the description excuses the review with a reason of its own."""
    return any(len(WORD.findall(m)) >= REASON_WORDS for m in EXCEPTION.findall(body or ""))


def exempt(pull: dict) -> bool:
    """A pull request Dependabot opened, whose bumps a reviewer cannot judge."""
    return (pull.get("author") or {}).get("login") in BOTS


def refusal(facts: dict, pull: dict) -> list[str]:
    """Why the merge path hands this pull request back; empty when it does not.

    `facts` is [`evidence`]'s answer. The first line is the verdict.
    """
    if not facts or exempt(pull) or excused(pull.get("body") or ""):
        return []
    found = hit(paths_of(facts.get("files") or []))
    level = needed(found)
    if level is None:
        return []
    done = recorded(facts.get("comments") or [])
    if done is not None and LEVELS.index(done) >= LEVELS.index(level):
        return []
    areas = sorted({area for area, _, _ in found})
    example = next(path for _, lvl, path in found if lvl == level)
    return [
        f"it changes {', '.join(areas)} (`{example}`) and has no independent"
        f" review at `{level}`."
        if done is None
        else f"its independent review ran at `{done}`, and {', '.join(areas)} asks for `{level}`.",
        "Run the `independent-review` skill and record it in a comment headed"
        f" `Independent review — {level}`, or excuse it on a line of the"
        " description: `No independent review: <why>`.",
    ]


def evidence(gh, repo: str, pull: dict, files: list[dict]) -> dict:
    """The pull request's changed files (already fetched) and its comments' bodies.

    The comments are asked for only when a trigger hits and nothing exempts or
    excuses the pull request, so most pull requests cost no extra call.
    """
    facts = {"files": files, "comments": []}
    paths = paths_of(files)
    if exempt(pull) or excused(pull.get("body") or "") or not hit(paths):
        return facts
    page = 1
    while True:
        batch = gh(
            "api", f"repos/{repo}/issues/{pull['number']}/comments?per_page=100&page={page}"
        )
        facts["comments"] += [item.get("body") or "" for item in batch]
        if len(batch) < 100:
            break
        page += 1
    return facts
