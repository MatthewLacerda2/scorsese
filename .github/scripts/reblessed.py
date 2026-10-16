"""Which reference images a pull request re-blesses, and whether it says why (#1217).

`docs/golden-renders.md`, *Re-blessing*: re-blessing a reference to make CI
green is never legitimate, and a re-blessed PNG in the diff is expected to be
explained in the description. The desktop app's panel snapshots follow the
same rule (`app/README.md`). Until this, nothing read the diff against the
description: the pixel gate was only as honest as whoever was fixing a red
golden at 3 a.m.

So `make mergeable` and `make queue` hand back a pull request that **modifies**
a reference without naming it under a `Re-blessed` heading or line with a
reason. An **added** reference needs nothing — a new fixture is not a
re-bless — and a removed one is not a re-bless either.

What satisfies it, in the description:

    ## Re-blessed
    - `alpha`, `blend`: the new dither moves every gradient by one level.
    - `page_*`: the Chromium pin moved to 155.0.1234.5.

or a single line, `Re-blessed: `title` — the title preset is two pixels
taller now.` A name is a golden fixture's directory name or a snapshot's file
stem; `prefix_*` covers every reference whose name starts with `prefix_`. The
line naming it has to carry a reason: at least [`REASON_WORDS`] words once the
names, paths and the word *Re-blessed* are taken out. **A machine can check
that a reason is written, not that it is sound** — that part stays review.

**The pin rule.** `tools/chromium/pin` names the one Chromium every page is
captured with, so moving it moves every `page_*` reference, and a pin move that
leaves one unchanged was never compared under the new browser. A pull request
that changes the pin must change (or add) a reference in every `page_*`
fixture at its head. Naming them is the rule above, and `page_*` does it once.

Pure but for [`evidence`], which asks GitHub over REST.
"""

from __future__ import annotations

import fnmatch
import re

GOLDEN = re.compile(r"^crates/golden/fixtures/([^/]+)/expected/[^/]+\.png$")
SNAPSHOT = re.compile(r"^app/tests/snapshots/([^/]+)\.png$")
PIN = "tools/chromium/pin"
FIXTURES = "crates/golden/fixtures"
PAGE = "page_"

# How many words a line has to keep, its names taken out, to count as giving a
# reason. Three is a short clause ("pin moved forward"), and more than a name
# with a colon after it.
REASON_WORDS = 3

HEADING = re.compile(r"^\s{0,3}#+\s")
MARKED = re.compile(r"re-?blessed", re.IGNORECASE)
LEAD = re.compile(r"^\s*(?:[-*+]\s+)?\**\s*re-?blessed\b", re.IGNORECASE)
PATTERN = re.compile(r"(?<![\w*/-])([A-Za-z0-9_]+)\*")
PATH = re.compile(r"\S*/\S*")
WORD = re.compile(r"[A-Za-z0-9]{2,}")


def name_of(path: str) -> str | None:
    """The reference a path is, by name, or `None` when it is not one."""
    for shape in (GOLDEN, SNAPSHOT):
        found = shape.match(path)
        if found:
            return found.group(1)
    return None


def modified(files: list[dict]) -> list[str]:
    """The references these files modify, by name, sorted and once each.

    `files` is GitHub's `pulls/N/files`. A rename that also changed the image
    is a modification under its new name; an added or removed one is not.
    """
    names = set()
    for item in files:
        status = item.get("status")
        if status == "modified" or (status == "renamed" and item.get("changes")):
            name = name_of(item.get("filename", ""))
            if name:
                names.add(name)
    return sorted(names)


def lines_marked(body: str) -> list[str]:
    """The description's lines that speak for re-blessed references.

    Every line under a heading carrying *Re-blessed*, up to the next heading,
    and any line that itself starts with it.
    """
    marked, inside = [], False
    for line in (body or "").splitlines():
        if HEADING.match(line):
            inside = bool(MARKED.search(line))
            continue
        if inside or LEAD.match(line):
            marked.append(line)
    return marked


def covers(line: str, name: str) -> bool:
    """Whether this line names the reference, outright or by `prefix_*`."""
    if re.search(rf"(?<![\w*-]){re.escape(name)}(?![\w*-])", line):
        return True
    return any(
        fnmatch.fnmatchcase(name, f"{prefix}*") for prefix in PATTERN.findall(line)
    )


def reasoned(line: str, names: list[str]) -> bool:
    """Whether the line still says something once names and paths are gone."""
    rest = PATH.sub(" ", line)
    rest = PATTERN.sub(" ", rest)
    for name in sorted(names, key=len, reverse=True):
        rest = re.sub(rf"(?<![\w-]){re.escape(name)}(?![\w-])", " ", rest)
    rest = MARKED.sub(" ", rest)
    return len(WORD.findall(rest)) >= REASON_WORDS


def unexplained(names: list[str], body: str, known: list[str]) -> list[str]:
    """The re-blessed references no marked line names with a reason.

    `known` is every name a line may mention, so that a list of names alone
    is not read as a reason made of names.
    """
    marked = lines_marked(body)
    return [
        name
        for name in names
        if not any(covers(line, name) and reasoned(line, known) for line in marked)
    ]


def unmoved(files: list[dict], pages: list[str]) -> list[str]:
    """The `page_*` fixtures a pin move left untouched; empty without one.

    `pages` is every `page_*` fixture at the pull request's head.
    """
    if not any(item.get("filename") == PIN for item in files):
        return []
    touched = {
        name_of(item.get("filename", ""))
        for item in files
        if item.get("status") in ("modified", "added", "renamed")
    }
    return [page for page in sorted(pages) if page not in touched]


def refusal(facts: dict, body: str) -> list[str]:
    """Why the merge path hands this pull request back; empty when it does not.

    `facts` is [`evidence`]'s answer. The first line is the verdict, as every
    refusal on the merge path.
    """
    files, pages = facts.get("files") or [], facts.get("pages") or []
    names = modified(files)
    known = names + pages
    silent = unexplained(names, body, known)
    stale = unmoved(files, pages)
    lines = []
    if silent:
        lines.append(
            f"it re-blesses {', '.join(f'`{n}`' for n in silent)} without saying why."
        )
        lines.append(
            "Name each under a `Re-blessed` heading or line, with the reason the"
            " new frames are right; `prefix_*` names a set. Re-blessing to make"
            " CI green is never legitimate (docs/golden-renders.md, *Re-blessing*)."
        )
    if stale:
        lines.append(
            f"it moves the Chromium pin but leaves {', '.join(f'`{p}`' for p in stale)}"
            " as they were."
        )
        lines.append(
            "Every `page_*` reference is captured with the pinned browser, so a"
            " pin move re-blesses all of them in the same pull request."
        )
    return lines


def evidence(gh, repo: str, pull: dict) -> dict:
    """The pull request's changed files and, if it moves the pin, its `page_*` fixtures."""
    files, page = [], 1
    while True:
        batch = gh(
            "api", f"repos/{repo}/pulls/{pull['number']}/files?per_page=100&page={page}"
        )
        files += batch
        if len(batch) < 100:
            break
        page += 1
    pages = []
    if any(item.get("filename") == PIN for item in files):
        listing = gh("api", f"repos/{repo}/contents/{FIXTURES}?ref={pull['headRefOid']}")
        pages = [
            entry["name"]
            for entry in listing
            if entry.get("type") == "dir" and entry.get("name", "").startswith(PAGE)
        ]
    return {"files": files, "pages": pages}
