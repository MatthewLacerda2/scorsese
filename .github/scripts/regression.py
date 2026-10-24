#!/usr/bin/env python3
"""Whether a bug fix carries a test that fails without it (#1508).

`docs/testing.md`, *A bug fix proves itself*: a pull request that closes an
issue labelled `bug` must add or change a test that **fails on the base** and
passes on the head. CI's `regression` job asks this script, in two steps:

    python3 .github/scripts/regression.py plan   # does this pull request owe one?
    python3 .github/scripts/regression.py run    # lay its tests over the base, run them

`plan` reads the description (`PR_NUMBER`'s, over REST, so a re-run sees an
edit) for closing keywords, the way `make mergeable` does
(`closing.referenced`), and asks GitHub which of those issues are labelled
`bug`. None, or an exception on a line of its own, and nothing is owed:

    No regression test on base: the bug was a real window's stutter.

Otherwise it lists the test files the pull request changed, as the size gate
classifies tests (`tools/lint/src/classify.rs`: a `tests` directory, a
`*.test.ts(x)`/`*.test.js`, a `test_*.py`) plus the golden fixtures, which are
the render layer's tests, and the targets those name. No target is a refusal.

`run` makes a worktree of the base (`HEAD^1`: CI checks out GitHub's merge
commit, whose first parent is the base's tip), copies every changed test file
and every changed file under a `fixtures` directory over it from the head
(deleting the ones the head deleted), and runs each target. One
target **failing** passes the check. A target that does not compile or import
on the base, or whose every failure is a name the base lacks (Python's
`AttributeError`, `NameError`, `ImportError`; bun's `… is not a function`),
does not count — a test calling the API the fix added would otherwise pass
anything — and neither does one that passes. A target past `TIMEOUT` (or
nextest's slow-timeout) has failed.

A target is the smallest thing each runner runs by name: a Cargo integration
test target, one Python test file, one `bun test` file. So an unchanged test in
the same target failing on the base would count too; `main` is green, so one
failing there is one the laid-over files broke.

Pure but for [`run_target`], [`lay_over`], [`labelled`], [`write_output`] and `main`.
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


def laid(path: str) -> bool:
    """What is copied over the base: every test file, and every file under a
    `fixtures` directory anywhere, which a test opens as data (a vendor's
    reply, a golden's project) but which names no target."""
    return is_test(path) or "fixtures" in PurePosixPath(path).parts[:-1]


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
    lines += [f"{t} {'passes on the base' if o == PASSED else JUDGED}." for t, o in sorted(results.items())]
    return lines + [HOW]


JUDGED = ("cannot be judged on the base: it does not build or import there, or fails"
          " only on a name the fix adds")
HOW = (
    "Write the test that reproduces the bug first, in a test file (a `tests`"
    " directory, `*.test.ts`, `test_*.py`) — docs/testing.md — or say why none"
    " can on a line of the description: `No regression test on base: <why>`."
    " A Playwright flow names no target: a web fix proved only by one adds a"
    " `bun test` too, or takes the exception. After editing the description,"
    " re-running this job is enough."
)


# One Python test file, run from the base worktree in a fresh interpreter so
# nothing of the head's scripts is imported. Exit 0 when every test passes, 1
# when one fails (a subtest included), 2 when it does not import, holds no
# test, or fails only on a name the base lacks: an `ImportError`, or a
# module's missing attribute, which is how a test reaching the fix's new API
# through a module it loaded fails. An `AttributeError` on an object (`None`
# has no attribute) is the bug itself, so it fails.
UNITTEST = """
import sys, unittest
from pathlib import Path
class Result(unittest.TextTestResult):
    missing = []
    def note(self, err):
        kind, value = err[0], err[1]
        Result.missing.append(issubclass(kind, ImportError)
                              or (issubclass(kind, AttributeError) and str(value).startswith("module ")))
    def addError(self, test, err):
        self.note(err); super().addError(test, err)
    def addFailure(self, test, err):
        self.note(err); super().addFailure(test, err)
    def addSubTest(self, test, subtest, err):
        if err is not None:
            self.note(err)
        super().addSubTest(test, subtest, err)
file = Path(sys.argv[1]).resolve()
top = file.parent
while (top / "__init__.py").exists():
    top = top.parent
loader = unittest.TestLoader()
suite = loader.discover(str(file.parent), pattern=file.name, top_level_dir=str(top))
if loader.errors or not suite.countTestCases():
    print("\\n".join(loader.errors))
    sys.exit(2)
result = unittest.TextTestRunner(verbosity=2, resultclass=Result).run(suite)
if not Result.missing:
    sys.exit(0 if result.wasSuccessful() else 1)
sys.exit(2 if all(Result.missing) else 1)
"""

EXITS = {PASSED: 0, FAILED: 1, BROKEN: 2}
# Each runner's per-target limit, nextest's own (`.config/nextest.toml`): a
# hung test is a failing one. A Cargo target gets its build's limit and its
# run's, since `tools/lint` has no nextest config to stop a hung test.
TIMEOUT = 180
CARGO_BUILD_TIMEOUT = 1800
CARGO_RUN_TIMEOUT = 900
# A bun failure that is a name the base lacks: an undeclared one. A
# `TypeError` (`… is not a function`, `undefined is not an object`) is also
# what a real bug throws, so it fails; a new export reached through a
# namespace import (`m.newFn()`) is a known limit (docs/testing.md).
BUN_MISSING = re.compile(r"^ReferenceError: \S+ is not defined")
BUN_ERROR = re.compile(r"^(?:\w*Error|error): .*$", re.M)


def bun_outcome(code: int, out: str) -> str:
    """A `bun test` run's outcome: failed when a `(fail)` test fails on
    anything but a missing name, broken when none ran or each failed that way."""
    if code == 0:
        return PASSED
    chunks = re.split(r"^\(fail\) .*$", out, flags=re.M)[:-1]
    if not chunks:
        return BROKEN
    # Each failing test's error is the last one printed above its `(fail)`.
    heads = [(BUN_ERROR.findall(chunk) or [""])[-1] for chunk in chunks]
    return BROKEN if all(BUN_MISSING.match(head) for head in heads) else FAILED


def nextest_outcome(code: int) -> str:
    """A `cargo nextest run` of a built target: 100 is a test failing or
    killed at its slow-timeout; anything else but 0 ran nothing it could judge."""
    return {0: PASSED, 100: FAILED}.get(code, BROKEN)


def run_target(t: Target, base: Path, env: dict) -> str:
    """Run one target inside the base worktree, its output to the log.

    Cargo builds into the head checkout's target directory, which CI's Rust
    cache warmed, so the base costs the workspace's own crates, not its
    dependencies. It runs under nextest, so the base's `.config/nextest.toml`
    (the `offline` setup script, the per-test timeout) applies."""
    if t.runner == "cargo":
        head = Path(env.get("REGRESSION_HEAD", base))
        env = {**env, "CARGO_TARGET_DIR": str(head / workspace(t.where, base) / "target")}

    def call(*argv: str, cwd: Path = base, limit: int, capture: bool = False) -> subprocess.CompletedProcess | None:
        """Run `argv`, its output streamed to the log as it comes unless it
        is `capture`d to be read, so a hang shows where it hung."""
        print(f"$ {' '.join(argv)}", flush=True)
        out = subprocess.PIPE if capture else None
        try:
            done = subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=out,
                                  stderr=subprocess.STDOUT, timeout=limit)
        except subprocess.TimeoutExpired:
            print(f"timed out after {limit}s", flush=True)
            return None
        if capture:
            print(done.stdout, flush=True)
        return done

    if t.runner == "cargo":
        manifest = ["--manifest-path", f"{t.where}/Cargo.toml", "--locked", "--test", t.name]
        built = call("cargo", "nextest", "run", *manifest, "--no-run", limit=CARGO_BUILD_TIMEOUT)
        if built is None or built.returncode:
            return BROKEN
        ran = call("cargo", "nextest", "run", *manifest, limit=CARGO_RUN_TIMEOUT)
        return FAILED if ran is None else nextest_outcome(ran.returncode)
    if t.runner == "unittest":
        ran = call(sys.executable, "-c", UNITTEST, t.name, limit=TIMEOUT)
        return FAILED if ran is None else {code: o for o, code in EXITS.items()}.get(ran.returncode, BROKEN)
    if not (base / t.where / "node_modules").exists():
        call("bun", "install", "--frozen-lockfile", cwd=base / t.where, limit=CARGO_RUN_TIMEOUT)
    ran = call("bun", "test", f"./{t.name}", cwd=base / t.where, limit=TIMEOUT, capture=True)
    return FAILED if ran is None else bun_outcome(ran.returncode, ran.stdout)


def workspace(package: str, root: Path) -> str:
    """The Cargo workspace a package builds in: the innermost directory at or
    above it whose `Cargo.toml` declares one (`.`, `app`, `tools/lint`), each
    with its own `target/`."""
    parts = PurePosixPath(package).parts
    for cut in range(len(parts), -1, -1):
        where = "/".join(parts[:cut]) or "."
        manifest = root / where / "Cargo.toml"
        if manifest.is_file() and "[workspace]" in manifest.read_text(encoding="utf-8"):
            return where
    return package


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
    paths = [p for p in changed(head, base) if laid(p)]
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


def description(gh: Callable, repo: str, number: str) -> str:
    """The pull request's description as it stands now, not as the event that
    started the run carried it: a re-run after an edit reads the edit."""
    return gh("api", f"repos/{repo}/issues/{number}").get("body") or ""


def main(argv: list[str]) -> int:
    step = argv[0] if argv else ""
    head, base = Path.cwd(), os.environ.get("BASE", "HEAD^1")
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        print("not a pull request: nothing is owed.")
        return write_output(False, [])
    repo = os.environ["REPO"]
    gh = labelled(repo, os.environ["GITHUB_TOKEN"])
    body = description(gh, repo, os.environ["PR_NUMBER"])
    fixed = bugs(closing.closed_by(gh, repo, {"body": body}))
    reason = excused(body)
    found = targets(changed(head, base), lambda p: (head / p).exists())
    if step == "plan":
        refused = owed(fixed, body, found)
        if refused:
            print("\n".join(["Refused:"] + refused))
            return 1
        if not fixed or reason:
            print(f"excused: {reason}" if fixed else "it closes no `bug` issue: nothing is owed.")
            return write_output(False, [])
        print("closes", fixed, "; runs on the base:", *map(str, found), sep="\n  ")
        return write_output(True, found)
    if step == "run":
        env = {**os.environ, "REGRESSION_HEAD": str(head)}
        refused = verdict(fixed, on_base(head, base, found, env))
        print("\n".join(["Refused:"] + refused) if refused else "a changed test fails on the base.")
        return 1 if refused else 0
    print("usage: regression.py plan|run", file=sys.stderr)
    return 2


def write_output(run: bool, found: list[Target]) -> int:
    """`run`, whether the job goes on, and `app`, whether a target is the
    desktop app's, whose libraries only then are installed."""
    app = any(t.runner == "cargo" and workspace(t.where, Path.cwd()) == "app" for t in found)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write(f"run={'true' if run else 'false'}\napp={'true' if app else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
