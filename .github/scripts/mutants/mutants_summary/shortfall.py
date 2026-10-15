"""When the report describes less than it was asked about."""

from __future__ import annotations


# ---------------------------------------------------------------------------
# When the report describes less than it was asked about
# ---------------------------------------------------------------------------
#
# Both callers can hand over a run that did not cover its subject, and for the
# same underlying reason: a clock ran out. The sweep meets it when a crate
# outgrows the six-hour job limit; an on-request run meets it when a scope is
# large enough that even sharded across several runners its mutants do not fit
# in the time each shard is given (#399). One vocabulary for both, because a
# reader of either has the same question — *what is this silent about?*
#
# The rule underneath is the one this repo applies to every signal: an absence
# must never render as a clean bill of health. A partial run is worth having;
# a partial run mistaken for a complete one is worse than no run at all.

CUT_SHORT = (
    "> **This run did not finish.** cargo-mutants recorded a start and no end,"
    " so it was stopped rather than completed — a time budget, a cancelled job,"
    " or the tool killed under it. What follows describes the mutants that"
    " *did* run, and not the whole of what was in scope."
)

# The size of the silence, in the only unit that means anything: mutations that
# were in scope and were never run. It is not the same claim as CUT_SHORT and
# both can appear — that one says the run stopped, this one says how much of the
# diff went unmeasured. Neither is a survivor and neither is a catch: what is
# known about these mutations is nothing at all, which is exactly why the number
# has to be printed rather than left to be inferred from a job that vanished.
GAP = (
    "> **{missing} of {in_scope} mutations in scope were not measured.**"
    " The report below describes {measured}. Nothing is known about the rest —"
    " they are neither caught nor survivors, and no absence of rows here says"
    " anything about them."
)


def cut_short(data: dict) -> bool:
    """Whether the run was stopped rather than finished.

    cargo-mutants writes `outcomes.json` as it goes and stamps `end_time` only
    on the way out, so a file with a start and no end is a run that was killed.
    Its running totals are real — they are just totals for a fraction of the
    subject. A run that recorded neither time is a fixture or an empty run, and
    claiming *that* was truncated would be inventing a failure.

    A sharded run inherits the same reading: `mutants/merge.py` gives the merged
    document an end only when every expected shard reported one, so one shard
    stopped by its budget makes the whole run read as stopped.
    """
    return bool(data.get("start_time")) and not data.get("end_time")


def measured(data: dict) -> int:
    """How many mutations this run actually reports an outcome for.

    Counted from the outcomes themselves and not from `total_mutants`, which is
    a statement about the work that was *planned*. The two differ for exactly
    the run this matters on. The baseline is skipped the same way `mutant_rows`
    skips it: its scenario is the string `"Baseline"` rather than a mutant, and
    counting it would overstate a sharded run by one per shard.
    """
    return sum(
        1 for outcome in data.get("outcomes", []) if isinstance(outcome.get("scenario"), dict)
    )


def shortfall(data: dict, in_scope: int | None) -> list[str]:
    """The banners this run owes its reader, in the order they are read."""
    said = [CUT_SHORT, ""] if cut_short(data) else []
    if in_scope is not None:
        done = measured(data)
        if in_scope > done:
            said += [GAP.format(missing=in_scope - done, in_scope=in_scope, measured=done), ""]
    return said
