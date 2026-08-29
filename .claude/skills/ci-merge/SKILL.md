---
name: ci-merge
description: Take a finished branch through to merged — gates, CI, the mutation signal, rebasing, and `make mergeable`. Use when a branch is ready, when a pull request has gone red, when a mutation report needs triage, or when merging anything into `main`.
---

# Getting a branch merged

The steps, and the traps. `CLAUDE.md` carries why these exist; this is how.

## Before marking a pull request ready

`make gates` must be green. It runs every gate CI blocks on — format, size, the
signal renderers, clippy, docs, tests, supply chain, and the desktop app's
workspace. `make help` lists them.

The app gate is the only conditional one, and it reports **skipped** when the
branch touches nothing under `app/`. Skipped is the honest answer; never read it
as green.

Deliberately **not** before every push. Checkpoint commits stay cheap — the
pre-commit hook is formatting and the size gate only, well under a second.

## The merge, one branch at a time

1. `git fetch origin && git rebase origin/main` in the branch's worktree.
2. Push with `--force-with-lease`.
3. Wait for CI **on the rebased head**.
4. `make mergeable PR=N`.
5. Merge. Then remove the worktree and delete the branch — a stale worktree is
   gigabytes.

Merging is serialized because Rust is compiled: two branches can each be green
alone and break `main` together. The only exception is a pull request touching
**only** Markdown, which CI skips. `docs/project-format.md` is not one of those —
tests parse its examples.

## `make mergeable` is the gate, and its answer is final

It asks GitHub whether a run genuinely happened on the head commit. `gh pr checks`
is **not** a substitute: it blends runs, so a skipped run hides behind a real one.

Three failure shapes it catches, all seen in practice:

- **A skipped run reading as green.** Several jobs appear twice, once SKIPPED and
  once SUCCESS. `make mergeable` names which run actually built the commit.
- **No run at all, because the pull request was readied moments after a push.**
  The checks are not green, they are absent. Force one with an empty commit.
- **No run at all, because the branch conflicts with `main`.** GitHub cannot
  build a merge ref for a conflicted branch, so it creates nothing — no run, no
  check, no error. This reads exactly like a broken workflow file.

**Telling the last two apart** — three lines, and `make mergeable` now says them
itself, so read its output before doing anything else:

1. **No run at all on a ready pull request → check whether the branch conflicts
   with `main`**, before touching a workflow file.
2. **An invalid workflow produces a run** — a `push`-event `startup_failure`.
   That is how the two are told apart. No run whatsoever means conflict.
3. **The fix is a rebase**, and it is the same rebase the merge routine above
   asks for anyway — so it costs nothing but doing it now.

**Never hand-roll a "wait for CI" loop that treats zero checks as success.**
Absent and passing are different states; a loop counting non-completed checks
finds zero of each. Require checks to **exist** before calling a run settled.

## A red ready pull request stays ready

Fixed in the next commit; it does not go back to draft. Draft is for work that is
genuinely unfinished, blocked, or handed over.

## Rebasing

Expect conflicts wherever every feature appends — an error list, a source enum, a
document type, a running record in a doc comment. **Two authors both being right
is the common case**, and the resolution is usually to keep both sides, ordered
deliberately rather than by merge accident.

Mechanical resolutions (a `mod` list, an import) are fine to do directly. Hand a
rebase back to the branch's author when resolving it needs to know *why* the code
is shaped as it is — a new variant that should join a documented grouping, two
prose paragraphs that need ordering, a signature that has grown a parameter.

After a rebase, re-check any claim the branch made **about the base it measured
against**. A byte-identity proof taken against an older `main` is stale, and
citing it is worse than not having run it.

## The mutation signal

It is a **signal, never a gate**. It cannot fail a build and **it does not hold a
merge**.

Read the report when it lists survivors in code **this branch wrote**. A report
with nothing in it, or whose survivors sit in untouched code, needs no reading.

Sort by cost:

- **Fix what is cheap** while the code is still in hand.
- **File a real bug** and fix it *after* the current branch merges.
- **File an architectural or foundational crack** — and do not start it without
  the user's judgement.
- Or **exclude it**, with a written reason, **line-qualified** so an unqualified
  entry cannot also swallow a real gap next door.

Stop the queue only for a report saying a **module** has nothing asserting its
mechanism at all. That is one finding about the tests, not a list of survivors.

**Establish equivalence by applying the mutation and running the suite** — never
by reasoning about symmetry. Reasoning has been wrong repeatedly; measurement has
not.

### Two patterns worth knowing before triaging

**A measurement that discards sign cannot test an operation that changes it.**
Magnitudes, DFT bins, peaks, absolute values, zero-crossing counts and
render-to-render equality all discard it, and every one reads like a real
assertion. A surviving `+` → `-` almost always means this. Four independent
modules hit it in one day.

**A boundary comparison surviving both `<=` and `>=` means neither end is
exercised.** Every instance so far has been a real gap — a first sample read as
silence, a value landing exactly on a limit belonging to neither side.

Also: a "0 of N caught, nothing at all" banner suggests the cause is structural
(assertions living in another crate). **Check before believing it** — it has been
wrong every time it appeared. `cargo mutants --list` names the functions and
builds nothing. The real causes have been ordinary: code with no test, and code
with no *callers* (which deletion fixes, not a test).

`make mutants` is opt-in and never part of passing. Do not run it while sibling
agents are compiling — it fans out and the machine cannot carry it.

## `SYNTH_VERSION`

Changing what a recipe renders to requires a bump, in the same commit. That rule
is in `CLAUDE.md` and is not negotiable.

**Verify by rendering only when the change touches rendering maths.** Build a
probe corpus and bake it against both checkouts when there is genuine doubt — an
edited note loop, a shared helper moved, a stage reordered. When the diff already
answers it — a new optional field defaulting to old behaviour, a variant nothing
existing can name — say so in one line and move on.

If you do run probes: **commit first**, then archive. An archive taken from a
dirty tree measures the wrong thing.
