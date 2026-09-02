---
name: issue-batch
description: Run a set of issues from board to merged — how many branches at once, which ones can safely run together, worktrees, and re-reading the board. Use when starting work on one or more issues, when deciding what to start next, or when told to "do the issues".
---

# Working a batch of issues

The user rarely has one issue. An idea becomes several, and more appear as coding
starts. This is how a set of them gets worked without the batch costing more than
the work.

## Two branches in flight, pipelined

Coding parallelises. **Merging does not** — Rust is compiled, so merges are
serialized, and the queue is the bottleneck.

Every branch that is not first pays a rebase for each merge ahead of it. Over
shared code that is **N(N−1)/2 rebases**: two branches cost one, three cost
three, four cost six. A rebase buys no correctness.

So: **one in the merge queue, one being written.** Nothing idles through a
ten-minute CI run, and nothing rebases twice.

## The real limit is file collision, not count

Two branches adding a variant to the same enum cost more than four branches in
genuinely separate areas. A clean rebase is seconds of `git`; a colliding one is
a whole session.

**Before starting a second branch, ask: does it edit the same types as the
first?** Three branches in `song/`, `fx/` and CI config barely touch each other.
Two branches both adding to a source enum will collide every time.

The files where everything collides are the ones every feature appends to — an
error list, a source enum, a pattern-entry type, a running record in a doc
comment. Two branches landing there at once is the case to avoid.

## Group the work before splitting it

**Split by responsibility, not by parallelism.** If a parent's sub-issues all
touch the same type, they are **one branch**, not one each.

Splitting an issue so several agents can run at once optimises the half that was
never scarce, and manufactures collisions: four sub-issues each adding a variant
to one enum is four rebases, four CI cycles and four mutation reports for one
coherent change.

Sub-issues are for work that is genuinely separable *in the code* — not for work
that is merely listable.

## Each branch gets its own worktree

One checkout per branch, never two branches taking turns in one. A shared
checkout mixes another issue's edits into `make gates` and thrashes `target/`.

**Never set `CARGO_TARGET_DIR`.** Cargo's default already gives every worktree
its own `target/`; an override makes worktrees overwrite each other's artifacts
and produces a false green. `make gates` refuses to run under one.

**Remove a worktree the moment its branch merges.** Each carries a full
`target/` — 8–17 GB apiece. Disposal is what keeps disk from becoming the
overnight failure, and it fails as a confusing build error rather than as "no
disk".

## Starting

- Assign the user the moment work begins — unassigned means fair game.
- **Unassign** if it turns out the issue was never started.
- Branch `{issue_number}-short-slug` off the latest `main`; an issue-less pull
  request uses a readable slug.
- Open a **draft** pull request on the first commit. Draft is how work survives a
  session that ends badly — the issue is the durable context, and a hand-back
  comment may never get written.

## Re-read the board after every merge

A merge changes the graph. Whatever the merged issue blocked is fair game the
moment it lands — so the decision is one merge wide, not one batch wide.

But re-reading is not a licence to start everything: **start the next one, and
keep the second slot for whatever is furthest along.** Priority orders what gets
merged. An unblocked issue left unstarted is not wasted capacity; it is a rebase
not yet paid for.

**A stage label is the only absolute stop.** `planning` and `human` mean
*not yet*, and no amount of the issue looking ready overrides that. Everything
else is startable the moment it exists, including an issue filed a minute ago.

## Briefing a subagent

Point it at `CLAUDE.md` first, then the issue — issues here are written to be
read cold. Beyond that:

- Name the **base commit** and what has landed recently that it must respect.
- Name the **siblings** and which files they are touching.
- Tell it to invoke the **`ci-merge` skill** rather than restating that protocol.
- Tell it **not** to merge — merging is serialized and belongs to the session
  running the batch.
- Tell it not to run `make mutants` while siblings are compiling.
- **Scratch filenames must carry the issue number.** The scratchpad is shared
  between sibling agents; a collision has already swapped one pull request's
  description for another's.

## Model, as a hint

Judgement work — design, implementation, triage — wants the strongest model. A
rebase, a module-list conflict, an attribute moved between files does not. Most
sessions on a branch are the second kind. The line is not crisp, so err upwards.

## When to hand back to the user

- ≈3 attempts at the same failure.
- A decision that is genuinely theirs: a format change, a name people will type,
  anything a `planning` label would have carried.
- Leave the pull request **draft**, say why in a comment, and stop. Do not thrash.

When working unattended, prefer leaving a comment on the issue and continuing
over stalling the night on a question.

## Reporting back

The user is not reading the transcript of a batch. They take long — often hours,
usually overnight — and the transcript is, if anything, notes for Claude itself.

**When things go well, say what the result was.** When things did not go as one
would expect, say what the surprise was. That does not necessarily mean things
went badly: we write it down because the more we can predict, the better we
improve.

Two things still interrupt, because they are the ones the user would want to
overrule and overruling is only possible while the batch is still running: **a
change to the user's own files** outside the repo, and **a decision reversed** —
where the issue said one thing and the branch did another.
