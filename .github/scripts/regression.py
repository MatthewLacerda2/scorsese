#!/usr/bin/env python3
"""Whether a bug fix carries a test that fails without it (#1508).

`docs/testing.md`, *A bug fix proves itself*: a pull request that closes an
issue labelled `bug` must add or change a test that **fails on the base** and
passes on the head. CI's `regression` job asks this script, in two steps:

    python3 .github/scripts/regression.py plan   # does this pull request owe one?
    python3 .github/scripts/regression.py run    # lay its tests over the base, run them

`plan` reads the description (`BODY`) for closing keywords, the way `make
mergeable` does (`closing.referenced`), and asks GitHub over REST which of
those issues are labelled `bug`. None, or an exception on a line of its own,
and nothing is owed:

    No regression test on base: the bug was a real window's stutter.

Otherwise it lists the test files the pull request changed, as the size gate
classifies tests (`tools/lint/src/classify.rs`: a `tests` directory, a
`*.test.ts(x)`/`*.test.js`, a `test_*.py`) plus the golden fixtures, which are
the render layer's tests, and the targets those name. No target is a refusal.

`run` makes a worktree of the base (`HEAD^1`: CI checks out GitHub's merge
commit, whose first parent is the base's tip), copies every changed test file
over it from the head (deleting the ones the head deleted), and runs each
target. One target **failing** passes the check. A target that does not
compile or import on the base does not count — a test calling the API the fix
added would otherwise pass anything — and neither does one that passes.

A target is the smallest thing each runner runs by name: a Cargo integration
test target, one Python test file, one `bun test` file. So an unchanged test in
the same target failing on the base would count too; `main` is green, so one
failing there is one the laid-over files broke.

Pure but for [`run_target`], [`lay_over`], [`labelled`] and `main`.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path, PurePosixPath
from typing import Callable, NamedTuple

sys.path.insert(0, str(Path(__file__).resolve().parent))

import closing  # noqa: E402

BUG = "bug"
EXCEPTION = re.compile(r"^\s*[-*]?\s*\**no regression test on base\**\s*:(.*)$",
                       re.IGNORECASE | re.MULTILINE)
WORD = re.compile(r"[A-Za-z0-9]{2,}")
# How many words an exception's reason keeps, as `reviewed.REASON_WORDS`.
REASON_WORDS = 3

# Test data outside a `tests` directory, and the Cargo target that reads it.
GOLDEN = ("crates/golden/fixtures/", "crates/golden", "goldens")

PASSED, FAILED, BROKEN = "passed", "failed", "broken"


class Target(NamedTuple):
    """What one runner runs by name: `cargo` (a package directory and a test
    target), `unittest` (a file) or `bun` (a package directory and a file)."""

    runner: str
    where: str
    name: str

    def __str__(self) -> str:
        if self.runner == "cargo":
            return f"`{self.where}` test target `{self.name}`"
        return f"`{self.where}/{self.name}`" if self.where else f"`{self.name}`"


def is_test(path: str) -> bool:
    """A test file as the size gate classifies one, or a golden fixture."""
    parts = PurePosixPath(path).parts
    name = parts[-1] if parts else ""
    return (
        "tests" in parts[:-1]
        or re.search(r"\.test\.(ts|tsx|js)$", name) is not None
        or (name.startswith("test_") and name.endswith(".py"))
        or path.startswith(GOLDEN[0])
    )


def target(path: str, exists: Callable[[str], bool]) -> Target | None:
    """The target a changed test file names, if it names one: a helper module
    or a fixture under `tests/` is laid over the base but runs nothing alone."""
    if path.startswith(GOLDEN[0]):
        return Target("cargo", GOLDEN[1], GOLDEN[2])
    parts = PurePosixPath(path).parts
    name = parts[-1]
    if name.startswith("test_") and name.endswith(".py"):
        return Target("unittest", "", path)
    if re.search(r"\.test\.(ts|tsx|js)$", name):
        for cut in range(len(parts) - 1, 0, -1):
            where = "/".join(parts[:cut])
            if exists(f"{where}/package.json"):
                return Target("bun", where, "/".join(parts[cut:]))
        return None
    if not name.endswith(".rs") or "tests" not in parts[:-1]:
        return None
    at = parts.index("tests")
    package, rest = "/".join(parts[:at]) or ".", parts[at + 1:]
    if not exists(f"{package}/Cargo.toml"):
        return None
    if len(rest) == 1:
        return Target("cargo", package, PurePosixPath(rest[0]).stem)
    if exists(f"{package}/tests/{rest[0]}/main.rs"):
        return Target("cargo", package, rest[0])
    return None


def targets(paths: list[str], exists: Callable[[str], bool]) -> list[Target]:
    """Every target the changed test files that the head still has name."""
    found = {target(p, exists) for p in paths if is_test(p) and exists(p)}
    return sorted(t for t in found if t is not None)


def bugs(closes: dict[int, dict]) -> list[int]:
    """The issues closed that are labelled `bug` (a pull request never is)."""
    return sorted(n for n, issue in closes.items()
                  if "pull_request" not in issue and BUG in closing.labels(issue))


def excused(body: str) -> str | None:
    """The reason the description gives for owing no regression test."""
    for reason in EXCEPTION.findall(body or ""):
        if len(WORD.findall(reason)) >= REASON_WORDS:
            return reason.strip()
    return None


def owed(fixed: list[int], body: str, found: list[Target]) -> list[str]:
    """`plan`'s refusal: a bug fix with no test target and no exception."""
    if not fixed or excused(body) or found:
        return []
    numbers = ", ".join(f"#{n}" for n in fixed)
    return [
        f"it closes {numbers}, labelled `{BUG}`, and changes no test that names a target.",
        HOW,
    ]


def verdict(fixed: list[int], results: dict[Target, str]) -> list[str]:
    """`run`'s refusal: no laid-over target failed on the base."""
    if any(outcome == FAILED for outcome in results.values()):
        return []
    numbers = ", ".join(f"#{n}" for n in fixed)
    lines = [f"no changed test fails on the base, so nothing shows {numbers} fixed."]
    lines += [f"{t} {'passes on the base' if o == PASSED else 'does not build or import on the base'}."
              for t, o in sorted(results.items())]
    return lines + [HOW]


HOW = (
    "Write the test that reproduces the bug first, in a test file (a `tests`"
    " directory, `*.test.ts`, `test_*.py`) — docs/testing.md — or say why none"
    " can on a line of the description: `No regression test on base: <why>`."
)


# One Python test file, run from the base worktree in a fresh interpreter so
# nothing of the head's scripts is imported: exit 2 when it does not import (or
# holds no test), 1 when a test fails, 0 when every one passes.
UNITTEST = """
import sys, unittest
from pathlib import Path
file = Path(sys.argv[1]).resolve()
top = file.parent
while (top / "__init__.py").exists():
    top = top.parent
loader = unittest.TestLoader()
suite = loader.discover(str(file.parent), pattern=file.name, top_level_dir=str(top))
if loader.errors or not suite.countTestCases():
    print("\\n".join(loader.errors))
    sys.exit(2)
sys.exit(0 if unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful() else 1)
"""

EXITS = {PASSED: 0, FAILED: 1, BROKEN: 2}


def run_target(t: Target, base: Path, env: dict) -> str:
    """Run one target inside the base worktree, its output to the log.

    Cargo builds into the head checkout's target directory, which CI's Rust
    cache warmed, so the base costs the workspace's own crates, not its
    dependencies."""
    if t.runner == "cargo":
        head = Path(env.get("REGRESSION_HEAD", base))
        env = {**env, "CARGO_TARGET_DIR": str(head / workspace(t.where, base) / "target")}

    def call(*argv: str, cwd: Path = base) -> subprocess.CompletedProcess:
        print(f"$ {' '.join(argv)}", flush=True)
        return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT)

    if t.runner == "cargo":
        manifest = ["--manifest-path", f"{t.where}/Cargo.toml", "--locked", "--test", t.name]
        build = call("cargo", "test", *manifest, "--no-run")
        print(build.stdout)
        if build.returncode:
            return BROKEN
        ran = call("cargo", "test", *manifest)
        print(ran.stdout)
        return PASSED if ran.returncode == 0 else FAILED
    if t.runner == "unittest":
        ran = call(sys.executable, "-c", UNITTEST, t.name)
        print(ran.stdout)
        return {code: o for o, code in EXITS.items()}.get(ran.returncode, BROKEN)
    if not (base / t.where / "node_modules").exists():
        print(call("bun", "install", "--frozen-lockfile", cwd=base / t.where).stdout)
    ran = call("bun", "test", f"./{t.name}", cwd=base / t.where)
    print(ran.stdout)
    if ran.returncode == 0:
        return PASSED
    return FAILED if re.search(r"^\(fail\) ", ran.stdout, re.MULTILINE) else BROKEN


def workspace(package: str, root: Path) -> str:
    """The Cargo workspace a package builds in: the outermost directory above
    it whose `Cargo.toml` declares one (`.`, `app`, `tools/lint`)."""
    parts, found = PurePosixPath(package).parts, package
    for cut in range(len(parts), -1, -1):
        where = "/".join(parts[:cut]) or "."
        manifest = root / where / "Cargo.toml"
        if manifest.is_file() and "[workspace]" in manifest.read_text(encoding="utf-8"):
            found = where
    return found


def git(*argv: str, cwd: Path | None = None) -> str:
    return subprocess.run(("git", *argv), cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE).stdout


def changed(head: Path, base: str) -> list[str]:
    """Every path the pull request changes, both ends of a rename."""
    return git("diff", "--name-only", "--no-renames", base, "HEAD", cwd=head).split()


def lay_over(head: Path, worktree: Path, paths: list[str]) -> None:
    """Copy the head's changed test files over the base, deleting the ones the
    head no longer has."""
    for path in paths:
        source, dest = head / path, worktree / path
        if source.is_file():
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)
        elif dest.is_file():
            dest.unlink()


def on_base(head: Path, base: str, found: list[Target], env: dict) -> dict[Target, str]:
    """Each target's outcome on the base with the head's test files laid over."""
    paths = [p for p in changed(head, base) if is_test(p)]
    with tempfile.TemporaryDirectory(prefix="regression-") as scratch:
        worktree = Path(scratch) / "base"
        git("worktree", "add", "--quiet", "--detach", str(worktree), base, cwd=head)
        try:
            lay_over(head, worktree, paths)
            return {t: run_target(t, worktree, env) for t in found}
        finally:
            git("worktree", "remove", "--force", str(worktree), cwd=head)


def labelled(repo: str, token: str) -> Callable:
    """`closing.closed_by`'s `gh`, over REST with the job's token."""
    def gh(_api: str, path: str) -> dict:
        request = urllib.request.Request(f"https://api.github.com/{path}", headers={
            "Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"})
        with urllib.request.urlopen(request, timeout=30) as reply:
            return json.load(reply)
    return gh


def main(argv: list[str]) -> int:
    step = argv[0] if argv else ""
    head, base = Path.cwd(), os.environ.get("BASE", "HEAD^1")
    body = os.environ.get("BODY", "")
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        print("not a pull request: nothing is owed.")
        return write_output(False)
    fixed = bugs(closing.closed_by(labelled(os.environ["REPO"], os.environ["GITHUB_TOKEN"]),
                                   os.environ["REPO"], {"body": body}))
    reason = excused(body)
    found = targets(changed(head, base), lambda p: (head / p).exists())
    if step == "plan":
        refused = owed(fixed, body, found)
        if refused:
            print("\n".join(["Refused:"] + refused))
            return 1
        if not fixed or reason:
            print(f"excused: {reason}" if fixed else "it closes no `bug` issue: nothing is owed.")
            return write_output(False)
        print("closes", fixed, "; runs on the base:", *map(str, found), sep="\n  ")
        return write_output(True)
    if step == "run":
        env = {**os.environ, "REGRESSION_HEAD": str(head)}
        refused = verdict(fixed, on_base(head, base, found, env))
        print("\n".join(["Refused:"] + refused) if refused else "a changed test fails on the base.")
        return 1 if refused else 0
    print("usage: regression.py plan|run", file=sys.stderr)
    return 2


def write_output(run: bool) -> int:
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write(f"run={'true' if run else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
