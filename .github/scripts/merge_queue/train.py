"""The merge train's tree: a chain of squash commits on `main`, one per member (#1757).

Ported from MatthewLacerda2/rusty's `merging/train/build.py` (its #1785), where
the design was settled; `yard.py` holds the rest of it and its module doc says
why the train is sound.

[`couple`] puts one pull request on the chain: the tree `git merge-tree` writes
for its head on the chain's tip, committed with the tip as its only parent.
Because the chain carries no commit of any branch, the merge base git finds is
the one GitHub's squash of that head onto `main` uses, so the chain's tip is
the tree `main` holds once every member is squash-merged in order. A head that
conflicts with the chain, or takes a number the chain already holds
([`taken`]: a migration's `NNNN_`, `SCHEMA_VERSION`, `SYNTH_VERSION`), stays
off it: git cannot say about those, so neither does the train.

`git(*args)` everywhere here returns `(status, stdout)`, as `merge-watch.py`'s
`_git` does, so a test answers it from a script.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import numbering
from .numbering import MIGRATIONS, SCHEMA_FILE, SYNTH_FILE


# Who the chain's throwaway commits are by. They never reach `main` (the squash
# merges do), and `commit-tree` refuses on a machine with no identity set.
IDENTITY = ("-c", "user.name=scorsese merge queue", "-c", "user.email=merge-queue@localhost")

# The branches a train is tested on: the queue's own throwaways, nobody else's.
PREFIX = "merge-train/"


@dataclass(frozen=True)
class Car:
    """One member: its number and head, and the chain's tree and commit once it is on."""

    number: int
    head: str
    tree: str
    commit: str


@dataclass
class Train:
    """The members on `base` (`origin/main` when it formed), in landing order."""

    base: str
    base_tree: str
    cars: list[Car] = field(default_factory=list)

    @property
    def tip(self) -> str:
        return self.cars[-1].commit if self.cars else self.base

    @property
    def tree(self) -> str:
        return self.cars[-1].tree if self.cars else self.base_tree

    @property
    def numbers(self) -> list[int]:
        return [car.number for car in self.cars]

    def tree_after(self, landed: int) -> str:
        """The tree `main` holds once the first `landed` members are merged."""
        return self.cars[landed - 1].tree if landed else self.base_tree


def start(git) -> Train | None:
    """An empty train on `origin/main` (fetched by the caller), `None` when git cannot say."""
    status, base = git("rev-parse", "--verify", "--quiet", "origin/main^{commit}")
    status_tree, tree = git("rev-parse", "--verify", "--quiet", "origin/main^{tree}")
    if status != 0 or status_tree != 0 or not base.strip() or not tree.strip():
        return None
    return Train(base.strip(), tree.strip())


def couple(train: Train, number: int, head: str, git) -> tuple[Car | None, str]:
    """`head` on the train's tip as GitHub's squash would land it, or `None` and why not.

    Fails closed, unlike the watch's conflict count: a member the train is not
    sure of stays off it, and takes its turn alone.
    """
    status, out = git("merge-tree", "--write-tree", "--no-messages", train.tip, head)
    tree = out.split("\n", 1)[0].strip() if status == 0 else ""
    if not tree:
        return None, "it does not merge cleanly onto the train so far"
    clash = taken(train, head, tree, git)
    if clash:
        return None, clash[0]
    status, out = git(*IDENTITY, "commit-tree", tree, "-p", train.tip, "-m", f"merge train: #{number} at {head[:12]}")
    commit = out.strip() if status == 0 else ""
    if not commit:
        return None, "git could not commit its tree onto the train"
    return Car(number, head, tree, commit), ""


def taken(train: Train, head: str, tree: str, git) -> list[str]:
    """[`numbering.numbering`] with the train's tip standing for `main`.

    `tree` is `head` merged onto the tip. Two members that each took the next
    number merge without a conflict, so only this sees them; the first one in
    line keeps the number, the second waits for a train of its own, where
    `rebase.collisions` hands it back to be renumbered.
    """
    tip = train.tip

    def listing(*args: str) -> list[str]:
        # The trailing slash names the files in the directory, not the directory.
        return git(*args, "--", f"{MIGRATIONS}/")[1].split()

    fork = git("merge-base", head, train.base)[1].strip()

    def versions(path: str, read) -> numbering.Versions:
        def at(rev: str) -> int | None:
            status, shown = git("show", f"{rev}:{path}")
            return read(shown if status == 0 and rev else None)

        return (at(fork), at(head), at(tree), at(tip))

    return numbering.numbering(
        listing("ls-tree", "--name-only", tip),
        listing("diff", "--name-only", "--diff-filter=A", tip, tree),
        versions(SCHEMA_FILE, numbering.schema_version),
        versions(SYNTH_FILE, numbering.synth_version),
    )


def halves(train: Train) -> list[list[tuple[int, str]]]:
    """A red train's two halves, each `(number, head)` in landing order."""
    cars = [(car.number, car.head) for car in train.cars]
    middle = len(cars) // 2
    return [cars[:middle], cars[middle:]]


def branch(train: Train) -> str:
    """`merge-train/<main>-<a>-<b>…`: one name per (main, members), readable in the Actions list."""
    return f"{PREFIX}{train.base[:7]}-{'-'.join(str(n) for n in train.numbers)}"
