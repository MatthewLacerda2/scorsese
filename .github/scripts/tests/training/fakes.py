"""A [`yard.Yard`]'s world as a script: git, GitHub and the clock faked (#1757).

[`git`] answers what [`train.couple`] asks the way git would: a head in
`clashing` conflicts with any tip, every other merge is clean, and the tree of
head `h` on tip `t` is named `t+h`, so a train's tree spells its members in
order. A head in `adds` adds those migrations, which the chain then holds.
[`Fx`] is the rest.
"""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("merge_queue_script", SCRIPTS / "merge-queue.py")
queue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(queue)

yard, group, rails, landing, merge = queue.yard, queue.yard.group, queue.yard.rails, queue.yard.landing, queue.merge
trains = queue.yard.trains
MIGRATIONS = "crates/server/migrations"


def git(clashing=(), adds=None):
    adds = adds or {}

    def migrations(rev: str) -> list[str]:
        return [path for part in rev.split("+") for path in adds.get(part, [])]

    def run(*args):
        args = [a for a in args if a not in trains.IDENTITY]
        if args[0] == "merge-tree":
            tip, head = args[-2], args[-1]
            return (1, "") if head in clashing else (0, f"{tip}+{head}\n")
        if args[0] == "commit-tree":
            return 0, f"{args[1]}\n"  # the commit is named after its tree
        if args[0] in ("rev-parse", "merge-base"):
            return 0, "main\n"
        if args[0] == "ls-tree":
            return 0, "\n".join(migrations(args[2]))
        if args[0] == "diff":
            tip, tree = args[3], args[4]
            return 0, "\n".join(p for p in migrations(tree) if p not in migrations(tip))
        return 1, ""  # `show`: no version constant in these trees
    return run


def pr(number: int, head=None) -> dict:
    return {
        "number": number, "headRefOid": head or f"h{number}", "author": {"login": "x"},
        "state": "OPEN", "isDraft": False, "labels": [{"name": "queue"}], "body": "",
    }


class Fx:
    """The effects, each recorded or scripted. `verdicts` is what each train's
    wait ends in, in turn (the last repeats); `squashes[n]` what merging `n` ends in."""

    def __init__(self, line=(), clashing=(), adds=None, files=None, refused=None, verdicts=(queue.GO,)):
        self.by = {p["number"]: p for p in line}
        self.verdicts, self.squashes, self.merged, self.said = list(verdicts), {}, [], []
        self.pushed, self.deleted, self.waited, self.main_moved, self.tree_override = [], [], [], 0, None
        self.push_refused = None
        files, refused = files or {}, refused or {}
        self.ns = argparse.Namespace(
            start=lambda: trains.Train("main", "main"), git=git(clashing, adds),
            files=lambda n: files.get(n, ["crates/core/src/lib.rs"]),
            judged=lambda pull: refused.get(pull["number"]), refusal=lambda pull: [],
            push=self.push, dispatch=lambda name: None, wait=self.wait,
            delete=self.deleted.append, look=lambda n: self.by[n], squash=self.squash,
            main_state=self.main_state, label="queue", no_merge=False,
            say=lambda *a: self.said.append(a[0]),
        )

    def yard(self):
        return yard.Yard(self.ns)

    def push(self, train):
        self.pushed.append(train.numbers)
        return self.push_refused

    def wait(self, sha, tick):
        self.waited.append(sha)
        verdict = self.verdicts.pop(0) if len(self.verdicts) > 1 else self.verdicts[0]
        return verdict, [f"the train {sha} is {verdict}."]

    def squash(self, number, sha):
        state = self.squashes.get(number, queue.MERGED)
        if state == queue.MERGED:
            self.merged.append(number)
        return state, f"{state}."

    def main_state(self, base):
        tree = "main" + "".join(f"+h{n}" for n in self.merged)
        return self.tree_override or tree, len(self.merged) + self.main_moved


def formed(fx: Fx, line: list[dict], at=None):
    members, train, notes = (at or fx.yard()).form(line)
    return [p["number"] for p in members], train.numbers if train else None, notes
