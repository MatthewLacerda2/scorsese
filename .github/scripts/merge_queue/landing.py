"""Landing a green train: each member in order, then `main`'s tree against the train's (#1757).

Ported from rusty's `merging/train/land.py`. Every member is read before the
first squash ([`departed`]): one that moved since the train formed (its head,
its label, its description) goes back in line and the rest are rebuilt
without it, nothing landed, so `main` never holds a tree CI did not run. What
can still end a landing partway is a move in the seconds between that read
and its squash, a refusal, or a failure in transit; the members after it are
untested without it, so they go back in line and are retested on the new
`main` ([`land`]).

Then [`check`]: `main` must hold exactly the members merged, one squash commit
each, and the tree the train built for that many. A merge of unknown fate
allows one more. Anything else stops the watch: it must never pass silently.
"""

from __future__ import annotations

from . import merge, mergeable, train as trains
from .states import HANDED_BACK, MERGED, REQUEUED, UNREACHABLE


MISMATCH = "main's tree is not the train's"


def moved(pull: dict, car: trains.Car, label: str, refusal) -> str | None:
    """Why `pull` may no longer land with its train, or `None`.

    `refusal(pull)` is the description's checks asked now (`described.py`): a
    `closes #N` added while the train ran would close the wrong issue.
    """
    if pull.get("state") != "OPEN":
        return f"it is {str(pull.get('state')).lower()} now"
    if pull.get("isDraft"):
        return "it went back to draft"
    if label not in mergeable.closing.labels(pull):
        return f"it lost the `{label}` label"
    if pull.get("headRefOid") != car.head:
        return f"its head moved to {str(pull.get('headRefOid'))[:7]} after the train tested {car.head[:7]}"
    refused = refusal(pull)
    return f"its description refuses it now: {refused[0]}" if refused else None


def departed(train: trains.Train, look, why_moved) -> list[tuple[int, str]]:
    """Each member that may no longer land with `train`, and why, read before
    the first squash. GitHub failing to show one counts it as departed:
    nothing lands on a doubt."""
    gone = []
    for car in train.cars:
        try:
            why = why_moved(look(car.number), car)
        except (mergeable.Unreachable, SystemExit) as down:  # SystemExit: `mergeable.Answered`
            why = f"GitHub did not show it: {down}"
        if why:
            gone.append((car.number, why))
    return gone


def land(train: trains.Train, look, why_moved, squash) -> tuple[list[tuple[int, str, str]], int, bool]:
    """Merge the members in order: the results, how many landed, and whether
    the last one tried has an unknown fate. `look(number)` is the pull request
    now; `squash(number, sha)` is [`merge.squash`]'s `(state, why)`."""
    results, landed, unknown = [], 0, False
    for at, car in enumerate(train.cars):
        state, why, unknown = step(car, look, why_moved, squash)
        if state == MERGED:
            results.append((car.number, MERGED, f"landed with the train {train.tip[:7]}."))
            landed += 1
            continue
        results.append((car.number, state, why))
        results += [
            (later.number, REQUEUED, f"not landed: #{car.number} did not land before it, so its train is untested without it.")
            for later in train.cars[at + 1:]
        ]
        break
    return results, landed, unknown


def step(car: trains.Car, look, why_moved, squash) -> tuple[str, str, bool]:
    """One member's landing: its state, why, and whether its fate is unknown."""
    try:
        why = why_moved(look(car.number), car)
    except (mergeable.Unreachable, SystemExit) as down:  # SystemExit: `mergeable.Answered`
        # Those before it landed and are kept; `check` still runs on them.
        return UNREACHABLE, f"GitHub did not show it before its squash: {down}", False
    if why:
        return REQUEUED, f"not landed with its train: {why}.", False
    state, why = squash(car.number, car.head)
    if state == MERGED:
        return state, why, False
    if state == merge.BASE_MOVED:
        return REQUEUED, f"not landed: {why}", False
    return HANDED_BACK, why, state == merge.UNKNOWN


def check(train: trains.Train, landed: int, unknown: bool, main_tree: str, squashes: int | None) -> str | None:
    """Why `main` is not what landing `landed` members of `train` must leave, or `None`.

    `main_tree` is `origin/main^{tree}` fetched after the landing; `squashes`,
    the first-parent commits from the train's base to it (`None`: git could
    not say).
    """
    counts = {landed, landed + 1} if unknown else {landed}
    trees = {train.tree_after(n) for n in counts if n <= len(train.cars)}
    if squashes not in counts:
        return (
            f"{MISMATCH}: {squashes} commit(s) landed on `main` since the train's base {train.base[:7]},"
            f" not the {landed} it merged; something else merged while it landed, so the tree it"
            " proved is not the tree `main` holds."
        )
    if main_tree not in trees:
        return (
            f"{MISMATCH}: after merging {', '.join(f'#{n}' for n in train.numbers[:landed])}, `main`'s tree is"
            f" {main_tree[:12]}, not the {train.tree_after(landed)[:12]} the train built. GitHub's merge and"
            " git's disagree; look at `main` before merging anything more."
        )
    return None
