---
name: issue-write
description: Write an issue for this repo — what it must contain, which labels it carries, and when Claude may file one unprompted. Use when filing an issue, splitting an idea into issues, or deciding whether something noticed mid-work deserves one.
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

Claude may open an issue autonomously, and should, for anything that will recur
or that a tool would solve more than once. Only when the benefit outweighs the
cost of building it.

The strongest issues come from doing the work: a mutation survivor that turned
out to be a real gap, a claim in a doc that quietly became false, a rule whose
suggested remedy misled. Those are findings, and findings are cheap to lose.

**File rather than fix** when the thing found is outside the branch in hand.
A branch that grows to cover everything it noticed is a branch nobody can review.

## Labels

**At most one stage label. Its absence means ready.**

- `idea` — might not add value; parked until the user decides. **Never started.**
- `planning` — has value, but the approach is still being discussed. **Never
  started.**
- `human` — needs a human in the loop end to end. **Treat as not-ready**: do not
  start it.
- *(none)* — anyone can tell an agent "do issue N".

**The judgement lives in the label**, so put it on honestly. Broad or vague is
what `planning` is for. A Claude-written issue **must** carry one of the three if
it is a breaking change, changes human-facing behaviour, needs a judgement call,
or proposes a structural change.

A `bug` usually should **not** carry one — it is specific, the deciding already
happened when the code broke, and nothing is gained by making it wait.

Type labels, combinable with a stage label:

`architecture` (communication structure, conventions, `project.json` format,
crate boundaries) · `infrastructure` (CI, harnesses, gates) · `bug` ·
`documentation` · `feature` (a capability serving the videos) · `foundation`
(groundwork making the editor more complete) · `human`.

## Priority

**architecture → infrastructure → bug → foundation → feature.** `documentation`
never waits its turn.

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
