"""The words the queue decides in, shared by every part of it.

Two vocabularies. The first is a single poll's answer — wait, go, stop, or
late — which [`verdict`], [`ci`] and [`merge`] all return. The second is how a
pull request's turn ended, which [`report`] sums up and `merge-queue.py`
hands to the summary.
"""

from __future__ import annotations


WAIT, GO, STOP, LATE = "wait", "go", "stop", "late"

# The two states a run nobody judged adds (#792). `RERUN` asks [`wait_for`] to
# re-run its failed jobs; `UNJUDGED` is that having not helped, which ends the
# turn *unfinished* exactly as the deadline does — nothing was decided.
RERUN, UNJUDGED = "rerun", "unjudged"

# What the summary calls each branch's ending. Merged and green are separate
# because `--no-merge` exists, and a queue that reported them the same would be
# claiming a merge it did not make. Unreachable is separate from handed back
# because it is not a verdict: GitHub stopped answering, nothing was decided,
# and the right response is to run the same queue again rather than to go and
# fix the branch (#615). Unfinished is separate from handed back for the same
# reason: a run still out at the deadline, or a pull request still in line when
# a watch ends, has had nothing decided about it (#690).
MERGED, GREEN, HANDED_BACK = "merged", "green", "handed back"
UNREACHABLE, NOT_REACHED, UNFINISHED = "unreachable", "not reached", "unfinished"

# The exit status when nothing was handed back and nothing was unreachable,
# but something is unfinished: run the queue again. Not 1, which says read a
# hand-back; not 3, which says GitHub stopped answering.
UNFINISHED_STATUS = 4

# The exit status when another queue holds this repository's lock (#1346):
# nothing was attempted, and the answer is to let that one finish.
LOCKED_STATUS = 5
