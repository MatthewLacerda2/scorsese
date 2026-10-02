#!/usr/bin/env python3
"""Turn the scope an agent names into a mutation run over exactly that.

`make mutants-remote SCOPE=…` hands one string to `mutants-on-request.yml`, and
this is what the workflow asks to make sense of it. Three spellings:

- **`diff`** — the lines this branch changed against `origin/main`, exactly the
  question `make mutants` asks locally.
- **a package name** (`scorsese-core`) — the whole crate, as the weekly sweep
  runs it. `cargo mutants -p` narrows before any mutant exists, so nothing
  from another crate comes along.
- **anything else** — one or more git pathspec globs, space-separated
  (`crates/core/src/keyframe/mod.rs`, `'crates/zimmer/src/song/**'`). `**`
  crosses directories and `*` does not, git's own `:(glob)` rules.

Why paths are not handed to cargo-mutants' own filters, which is the whole of
this file's reason to exist: none of them narrows for real.
`docs/mutation-testing.md` (*Running it*) has the detail — `--file` is unioned
with the config's `examine_globs`, so it *widens*; `--re` narrows by name but
struct-field deletions ignore it, so a run asked about one file still carries
every `delete field` mutant on the surface (26 of them, on the first file this
was tried against, and not one from the file asked about). What does narrow
exactly is `--in-diff`: it keeps only mutants whose span touches a changed
line, and a diff that *adds* the named files whole touches every line of them
and of nothing else. So a path scope becomes a diff — the files as new, from
`/dev/null` — and all three spellings end up as one of two cargo-mutants
arguments, both of which narrow before anything is built.

The narrowing is still checked rather than trusted. `check` reads what
`cargo mutants --list --json` planned against the files the scope named and
says, in the report's scope line, how many planned mutations came from
anywhere else. It should be none; if a later cargo-mutants changes what
`--in-diff` lets through, the report says so instead of describing files
nobody asked about as if they had been.

    mutants-scope.py resolve 'crates/core/src/keyframe/**' --out scope.json --diff scope.diff
    cargo mutants --list --json $(jq -r '.args[]' scope.json) > planned.json
    mutants-scope.py check scope.json planned.json > scope.txt
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

#: The base a `diff` scope is measured against: the branch the merge queue
#: merges into, and the one `make mutants` scopes itself to.
BASE = "origin/main"


def git(*args: str, ok: tuple[int, ...] = (0,)) -> str:
    """`git` with its stdout, dying with git's own words on anything else."""
    done = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
    if done.returncode not in ok:
        sys.exit(f"mutants-scope: git {' '.join(args)}: {done.stderr.strip()}")
    return done.stdout


def packages() -> set[str]:
    """The workspace's package names, which is what a crate scope must be."""
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {package["name"] for package in json.loads(out)["packages"]}


def kind(scope: str, known: set[str]) -> str:
    """Which of the three spellings `scope` is. Empty is an error, not a default.

    A crate name is recognised by being one, not by its prefix: a mistyped
    crate falls through to being a path, matches no file, and is refused there
    with the path in the message — which is a better error than guessing.
    """
    words = scope.split()
    if not words:
        sys.exit("mutants-scope: the scope is empty -- name `diff`, a crate, or paths")
    if words == ["diff"]:
        return "diff"
    if len(words) == 1 and words[0] in known:
        return "crate"
    return "paths"


def matching(globs: list[str]) -> list[str]:
    """The tracked Rust files the globs name, in git's own glob dialect."""
    listed = git("ls-files", "-z", "--", *(f":(glob){g}" for g in globs))
    return sorted(p for p in listed.split("\0") if p.endswith(".rs"))


def whole(files: list[str]) -> str:
    """A diff that adds each file entire, so `--in-diff` keeps all of it.

    `git diff --no-index` exits 1 when the two sides differ, which against
    `/dev/null` they always do; that is the answer and not a failure.
    """
    return "".join(git("diff", "--no-index", "--", "/dev/null", f, ok=(0, 1)) for f in files)


def branch_diff(base: str) -> tuple[str, list[str]]:
    """What this branch changed in Rust since it left `base`."""
    if not git("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}", ok=(0, 1)).strip():
        sys.exit(f"mutants-scope: {base} is not in this checkout; a diff against it would be empty")
    fork = git("merge-base", base, "HEAD").strip()
    names = git("diff", "--name-only", fork, "HEAD", "--", "*.rs").split()
    return git("diff", fork, "HEAD", "--", "*.rs"), sorted(names)


def resolve(scope: str, known: set[str], base: str, diff_path: str) -> tuple[dict, str]:
    """The run this scope asks for, and the diff file it needs (maybe empty)."""
    which = kind(scope, known)
    if which == "crate":
        return {"kind": which, "scope": scope, "args": ["-p", scope], "files": None}, ""
    if which == "diff":
        diff, files = branch_diff(base)
        said = f"the Rust this branch changed against `{base}`"
    else:
        files = matching(scope.split())
        if not files:
            sys.exit(f"mutants-scope: no tracked Rust file matches {scope!r}")
        diff, said = whole(files), None
    return {
        "kind": which,
        "scope": scope,
        "args": ["--in-diff", diff_path],
        "files": files,
        "said": said,
    }, diff


def outside(plan: dict, planned: list[dict]) -> list[dict]:
    """Planned mutants from somewhere the scope did not name."""
    if plan["kind"] == "crate":
        return [m for m in planned if m.get("package") != plan["scope"]]
    named = set(plan["files"])
    return [m for m in planned if m["file"] not in named]


def check(plan: dict, planned: list[dict]) -> tuple[str, bool]:
    """The report's scope line, and whether the run should go ahead at all.

    Nothing planned is an answer for a `diff` (a branch that only touched
    tests) and an error for anything named: a crate or a path that is not on
    the mutated surface is a request this run cannot answer, and reporting
    zero survivors for it would be the clean bill of health over an absence
    this repo's signals exist to refuse.
    """
    count = len(planned)
    noun = "mutation" if count == 1 else "mutations"
    if plan["kind"] == "crate":
        subject = f"`{plan['scope']}`, the whole crate"
    elif plan["kind"] == "diff":
        subject = f"{plan['said']} ({len(plan['files'])} files)"
    else:
        shown = ", ".join(f"`{f}`" for f in plan["files"][:5])
        more = f" and {len(plan['files']) - 5} more" if len(plan["files"]) > 5 else ""
        subject = f"{shown}{more}, matched by `{plan['scope']}`"
    line = f"Scoped on request to {subject}: {count} {noun} planned."
    stray = outside(plan, planned)
    if stray:
        files = sorted({m["file"] for m in stray})
        line += (
            f" {len(stray)} of them are from outside that scope"
            f" ({', '.join(f'`{f}`' for f in files[:5])}"
            f"{' and more' if len(files) > 5 else ''}) and are reported anyway;"
            " read those rows as not yours."
        )
    if count == 0 and plan["kind"] != "diff":
        return line + " None of it is on the mutated surface (`.cargo/mutants.toml`).", False
    return line, True


def main() -> None:
    parser = argparse.ArgumentParser(prog="mutants-scope.py", description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    res = sub.add_parser("resolve", help="turn a scope into cargo-mutants arguments")
    res.add_argument("scope", help="`diff`, a package name, or space-separated path globs")
    res.add_argument("--base", default=BASE, help="what a `diff` scope compares against")
    res.add_argument("--out", type=Path, required=True, help="where the plan JSON goes")
    res.add_argument("--diff", type=Path, required=True, help="where the scope's diff goes")
    chk = sub.add_parser("check", help="the scope line, from what --list planned")
    chk.add_argument("plan", type=Path, help="the JSON `resolve` wrote")
    chk.add_argument("planned", type=Path, help="`cargo mutants --list --json` output")
    args = parser.parse_args()

    if args.command == "resolve":
        plan, diff = resolve(args.scope, packages(), args.base, str(args.diff))
        args.out.write_text(json.dumps(plan))
        args.diff.write_text(diff)
        print(" ".join(plan["args"]))
        return
    # cargo-mutants prints nothing at all, not `[]`, when nothing is in scope.
    listed = args.planned.read_text().strip()
    line, go = check(json.loads(args.plan.read_text()), json.loads(listed) if listed else [])
    print(line)
    if not go:
        sys.exit(1)


if __name__ == "__main__":
    main()
