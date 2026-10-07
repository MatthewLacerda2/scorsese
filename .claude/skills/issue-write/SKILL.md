---
name: issue-write
description: Write an issue for this repo — what it must contain, which labels it carries, and when Claude must file one unprompted. Use when filing an issue, splitting an idea into issues, or deciding whether something noticed mid-work is folded in or filed, and with which labels.
---

# Writing an issue

The unit of work here is a well-specified issue. A future Claude reads it **cold**
and says *"I understand the assignment, I know how to proceed."* That is what lets
an issue run unattended, overnight, with nobody to ask.

## What it contains

- **What** — the change, concretely.
- **Why it belongs** — the argument. This is the half that survives; an issue
  whose reasoning is written down can be re-judged when circumstances change, and
  one without it can only be obeyed or ignored.
- **The roadmap** — the shape of the work, *not* the implementation intrinsics.
  Name the decisions the implementer must make and leave them theirs. Say what is
  explicitly **out of scope**; a boundary stated once saves an argument later.

**Evidence beats assertion.** An issue that quotes a measurement, a failing
report, a real file on disk or a line of the codebase is one nobody has to
re-derive. "The bake report prints `low 61%` and the only lever is a fader" is
worth more than "we should have an EQ".

**Point with symbols and paths, not line numbers.** `AssetKind::is_prompted`
or `merge-queue.py`'s `push_needed` survive the edits that land before the issue
is picked up; a bare `crates/core/src/project.rs:210` does not. On rusty, on
2026-09-30, a batch merged ~50 pull requests in a day, a central file was split
into a module in the middle of it, and nearly every brief had to warn its coder
that the issue's line references were stale (MatthewLacerda2/rusty#586). A line
number is fine as a hint *beside* a symbol, never on its own.

**Cite what it relates to.** Sibling issues, the pull request that exposed it, the
rule in `CLAUDE.md` it turns on. A future reader arrives with no memory of today.

## The three gates

An idea becomes an issue only when all three hold. If any fails, **push back
instead of complying**:

1. **Understanding** — the intent is clear and can be restated. If unsure,
   restate it and confirm; do not guess.
2. **Value** — real value to the project. No busywork, no features for their own
   sake.
3. **Craft** — Rust good practice and the decided architecture. If the idea
   violates a settled decision, say so and propose the right shape.

## Filing what you notice

Filing is a **duty**, not an option (CLAUDE.md, *File what you notice*, #757):

- **Nothing you find is dropped** — a bug, a missing feature, a design gap, a
  quality-of-life improvement. Bugs are not a class apart: each is folded into
  the task at hand or filed, and which is your call.
- **A change that needs the user's approval is never folded in**: one that
  changes how the user sees or understands their existing data or project,
  changes stored data, or needs a migration. Put its decisions to the user and
  file it once they are settled (*Labels* below); found unattended, file it with
  `planning`.
- **Anything else needs no stage label** and is startable at once — a clearer
  control, a new read-only view, a fix that touches no stored data, a gap the
  design docs already say how to close.
- **Label it `agent`** whenever you wrote it on your own initiative rather than
  because a person asked for it.

The strongest issues come from doing the work: a mutation survivor that turned
out to be a real gap, a claim in a doc that quietly became false, a rule whose
suggested remedy misled. Those are findings, and findings are cheap to lose.

**File rather than fix** when the thing found is outside the branch in hand.
A branch that grows to cover everything it noticed is a branch nobody can review.

## Labels

**At most one stage label. Its absence means ready.**

- `planning` — the user has not finished understanding and agreeing on a
  design or architecture. **Never started.**
- `human` — needs a human in the loop end to end. **Treat as not-ready**: do not
  start it.
- *(none)* — anyone can tell an agent "do issue N".

**Stage labels are the user's to ask for** (CLAUDE.md, *Stage labels*,
2026-10-07). An issue that is a breaking change, needs a judgement call,
proposes a structural change, or meets the approval test above (changes how the
user sees or understands their existing data or project, changes stored data, or
needs a migration) is not parked under a label. Instead:

1. **State the problems and the decisions they need**, in plain words, each with
   your recommendation and its reason. Ask them together, not one per turn.
2. **Write the answers into the issue as decided**: what was chosen, by whom,
   when, and why, so a coder reading it cold has nothing left to ask.
3. **File it with no stage label.** It is startable at once.

Put `planning` or `human` on only when the user asks for it (usually: they are
out of time to agree on the design now). **The one exception you apply
yourself:** a finding made while working unattended (a batch, a cloud session,
overnight) that needs the user's approval is filed with `planning`, because
nobody has agreed to it yet. List its open decisions in the issue, each with a
recommendation, and raise them at the next check-in.

A bug is held to that same test, not a looser one: most need no decision,
because the deciding happened when the code broke; a bug whose fix needs a
migration or rewrites stored data still goes to the user first.

Type labels, combinable with a stage label:

`architecture` (communication structure, conventions, `project.json` format,
crate boundaries) · `infrastructure` (CI, harnesses, gates) · `bug` ·
`documentation` · `feature` (a capability serving the videos) · `foundation`
(groundwork making the editor more complete) · `human`.

`agent` marks an issue an agent wrote on its own initiative (#757). It combines
with any type and stage label and is neither: it never stops work, it only tells
the user who decided this was worth filing.

## Priority

**infrastructure → architecture → bug → foundation → feature.** `documentation`
never waits its turn. A bug in the development tooling itself — CI, the gates,
the hooks, `make queue` / `make mergeable` — ranks as `infrastructure`, whatever
its label.

Priority orders what gets **merged**, not what gets **worked**.

## Relationships

Use GitHub's **Blocked by / Blocks**, and **sub-issues** when one is literal
groundwork for another. Link when one lays groundwork, makes the next
meaningfully easier, or would conflict too much if done concurrently.

**The dependency graph is the plan** — there are no rigid batches.

**Do not split for parallelism.** Sub-issues that all touch the same type are one
issue; see the `issue-batch` skill for why that costs more than it saves.

If a `planning` issue would affect how another is implemented or thought of, mark
that other one **blocked by** it.

## Closing

Reference the issue from the pull request that closes it — and **check the number**.
A typo'd `Closes #N` closes the wrong issue or none, silently, and nothing
verifies it. Work has sat "open" for days that way.

The same failure from the other side: **GitHub's closing keywords match anywhere
in a pull request's description**, inside backticks, inside quotes, and inside a
sentence that says the opposite. A PR that must leave an issue open writes
`refs #N` or `part of #N`, and never contains `closes`, `fixes` or `resolves`
before that number in any form (rusty found this when a description explaining
it did *not* close an issue closed it on merge, MatthewLacerda2/rusty#817).
