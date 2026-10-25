---
name: issue-batch
description: Run a set of issues from board to merged — how many branches at once, which ones can safely run together, worktrees, cloud coders, re-reading the board, turning lessons into changes, and the cleanup and local rebuild that finish a batch. Use when starting work on one or more issues, when deciding what to start next, or when told to "do the issues".
---

# Working a batch of issues

An idea becomes several issues, and more appear as coding starts. This is how
a set of them gets from the board to merged without the batch costing more
than the work. The premise is one person, on their own machine or in a cloud
session; it is up for review the day there are other contributors.

**Write a lesson here as the rule, not the story.** A date and a pull request
number earn their place only where the rule would be doubted without them, and
a rule a tool now enforces leaves this file. Before adding to it, take out what
the tooling or a better model has made unnecessary.

## The queue is the bottleneck

Coding parallelises; merging does not. Rust is compiled, so merges are
serialized, one CI run each, and every branch behind another pays a rebase per
merge ahead of it — N(N−1)/2 over shared code. What matters is that the queue
never idles and nothing rebases twice, not how many branches are being written.

**The merge queue is `make queue WATCH=1`** (`ci-merge` has how it orders,
hands back and retakes). It takes every ready pull request labelled `queue`,
so feeding it is one label per pull request once you have read it, and the
merge order lives on GitHub. When the batch is scoped and another session's
pull requests already carry `queue`, feed it fixed lists instead (`make queue
PRS="a b c"`), since the watch takes every labelled one.

## How many agents at once

- **Cloud coders: about ten in flight.** The default home for writing a branch:
  it brings its own CPU, memory and disk. Ten paces the token spend and what
  one orchestrator can brief and read; size it from what is actually running.
- **Local coders: at most two.** For what needs this machine (*Where a branch
  runs*). The queue merges one per CI run however many are written, and this
  machine also hosts the web app.
- **Agents that only read and think: no limit.** Research, review, planning:
  no worktree, no `target/`.

The orchestrator is one more, always.

## The real limit is file collision, not count

Two branches adding a variant to one enum cost more than four in separate
areas. Before starting a branch, ask whether it edits what an in-flight one
edits. A crate is too coarse a unit — nearly every editing feature touches
`core` and `compositor` — so sort by the shared appends: validation errors, the
per-layer compositing path, the format doc, the schema bump. Files that are not
compiled collide too (`web/` components). The usual collisions:

- **A file at the size cap.** Every branch appending to it must first make
  room, and each makes it differently. When a merge leaves a shared file within
  a few lines of the cap, say so in the next `Brief` of every sibling touching it.
- **Lockfiles have one owner per wave.** A lockfile conflict is a regeneration,
  a locked check and `cargo deny`, not a quick rebase. While a branch changes
  dependencies broadly (an open Dependabot pull request included), it is the
  only one in its wave touching manifests or lockfiles; a sibling needing a
  dependency waits for it, or takes the dependency through it.
- **Numbers.** `schema_version`, `SYNTH_VERSION` and a migration's `NNNN_` are
  taken last, from `main` + 1, and the queue hands back one already taken. When
  you tell coders a number, hand it out in landing order: the second fails the
  migrations' no-gaps test until the first has landed. A schema bump also turns
  on the `web` and `app` gates.
- **Reference images.** Two branches re-blessing the same golden or app
  snapshot conflict in binary, and the second re-blesses on the new `main`. Put
  them in one lane, or plan the re-bless as a step of the second.
- **A new gate breaks the branches already in line.** A pull request adding a
  check proves only its own `main` clean. When it is next to merge, run its
  check over the queued heads and fix them before their turn.

**Split by responsibility, not by parallelism.** Sub-issues that all touch the
same type are one branch; sub-issues are for work separable in the code, not
merely listable.

## Local worktrees and builds

One worktree per branch, off the latest `main`, removed the moment it merges:
each `target/` reaches ~40 GB after `make gates`, and a full disk fails as a
confusing build error. Never set `CARGO_TARGET_DIR` (`make gates` refuses one
outside the worktree). `make setup` shares sccache's cache between worktrees
when it is installed: about two minutes off a cold worktree, nothing off an
incremental build.

The operator works on more than one machine, so measure the one you are on:

- A cold build or `make gates` fills every core. A second one at the same time
  exists so one agent is not queued behind another, never for throughput; give
  each half the cores (`CARGO_BUILD_JOBS`). A third buys nothing.
- Check free memory and disk first. Running out shows up as a link failure or a
  killed rustc, not as "no disk".
- The machine that hosts the web app shares its cores with paying users'
  renders: leave it headroom.
- Short of disk, a git-excluded `.cargo/config.toml` with `[build] incremental
  = false` shrinks a worktree at the price of slow warm rebuilds.

## Cloud coders

**Where a branch runs.**

- **Local:** the orchestrator and every merge; anything needing the operator's
  files or keys (`make live-check` spends money and never runs in the cloud); a
  real display, GPU or speakers; anything macOS-specific; a measurement that
  decides something (and nothing else compiles beside it).
- **Cloud:** everything else whose proof is `make gates`. The container has
  Postgres and can start `dockerd`, so `crates/server` and `deploy/` work there,
  and it is x86_64 Linux like CI, so it is where a new golden fixture is blessed
  (never on arm64, even in a Linux container: thin strokes bless wrong).

**Launching one** is a one-off routine, `RemoteTrigger create` (the `schedule`
skill has the body), not the `Agent` tool's `isolation: "remote"`, which falls
back to a local worktree in a local session. Give it:

- the **`anthropic_cloud` environment**, never a `bridge` one: a bridge runs
  the routine on the operator's own machine;
- `run_once_at` computed, never typed: `date -u -d '+2 min'
  +%Y-%m-%dT%H:%M:00Z` (`-v+2M` on macOS); the API refuses a time already past;
- the model *Model* names (the `schedule` example defaults to a Sonnet), and
  `allowed_tools` with room for ToolSearch and the GitHub MCP tools;
- this repository as its source, and this prompt:

```text
Read `.claude/skills/issue-batch/cloud-brief.md` and follow it.
Issue(s): #N. Your brief is the newest comment on it that starts with `Brief`.
```

Creating routines is rate-limited, about four at once, then `429 … Try again
in 15s`: wait what it names, with `run_once_at` far enough out to survive it.

**The brief is a comment on the issue**, never the prompt: `RemoteTrigger`
echoes a prompt back four times into this context, and a comment survives for
whoever resumes. The standing rules every coder follows are `cloud-brief.md`'s;
a change to them goes there, once.

```text
Brief

Branch: `N-short-slug`.
Builds on: <what merged that it must build on>.
Siblings in flight: <branch/issue → the files it edits; stay out of them>.
Decisions: <anything already settled, or "none">.
```

A re-brief (a rebase request, a new sibling to avoid) is a new `Brief` comment
and a fresh routine.

**The pull request is the report.** A coder cannot message you; the queue's
watch tells you when its pull request turns ready. Ready is the coder's claim
that it is finished, and from then the branch is the queue's: a red run comes
back to you as a hand-back, to fix or re-brief.

**A stalled coder** shows as `worker_status: idle` on `list_runs` with a draft
pull request, but read its log first: idle with a background task pending is
waiting. Idle with nothing pending is stuck: fire a fresh routine briefed to
finish the pushed branch.

**The usage limit stops every coder at once**, since they run on the
orchestrator's account. When it resets, read each run's log tail and what each
branch pushed, post a new `Brief` ("continue PR #N from its pushed head", or
"start fresh", with what the first attempt learned), and `RemoteTrigger run`
the same routine.

## Running a mixed batch: the orchestrator's loop

The orchestrator is the batch's own session, local. It writes no feature code:
it picks work, briefs coders, reviews and merges.

**One background watch.** Start the queue as a background command with its
output in a file, and a `Monitor` on its events:

    make queue WATCH=1 SINCE=<batch start, ISO 8601> > <scratch>/queue-watch.log 2>&1
    tail -F <scratch>/queue-watch.log | grep --line-buffered '^watch: '

Never the watch itself under `Monitor`, which is killed at thirty minutes while
the watch runs seventy, and never behind `| tail`, which shows nothing until it
exits. Each `watch:` line is an event to act on. The last is `watch: resume:
<command>`: start exactly that again the same way, and re-arm the Monitor when
it expires. Exit status 5 means another queue holds the lock: never start a
second. Stop a watch by the PID you hold, never `pkill -f`, since every
repository the operator batches runs the same script.

For each pull request that turns ready:

1. **Read the description and the diff.** A green gate does not read code.
   Check its decisions against the issue, and its *Gates* line: a cheaper model
   has readied a pull request without running `make gates`. A Sonnet's pull
   request carries `minor`; add it if not. A risky change (money, stored data,
   security, concurrency, the render pipeline, the gates) goes through
   **`independent-review`** before the label.
2. **Label it `queue`.** When merges ahead touched the same crates, first
   rebase it yourself, `cargo check --workspace --all-targets --locked` (and
   `--manifest-path app/Cargo.toml` when it touches `app/`) and `make size`, and
   push: a clean rebase can still break a call (`ci-merge`). For a cloud-written
   branch, do that only when a merge ahead changed a signature it calls.
3. **A hand-back** is fixed on the branch or re-briefed; the push re-queues it.
4. **After a merge**, remove its worktree, re-read the board and start the next
   piece of work.

**GitHub is the batch's state.** Open pull requests, their comments and
`RemoteTrigger list` are enough to resume from nothing. Never keep anything
only in `/tmp`: a reboot wipes it.

## Starting, and re-reading the board

Assign the user when work begins, and unassign if it never started. Branch
`{issue}-short-slug` off the latest `main` and open a draft pull request on the
first commit.

A merge changes the graph: whatever it blocked is startable now. **Re-read the
whole board** (`gh issue list`), never your list of it: coders and the operator
file mid-batch, and a new label-priority leader is started next. But start
within the limits above: an unstarted issue is a rebase not yet paid for.
`planning` and `human` are the only stop.

Dependabot's pull requests label themselves `queue` and need nothing, except
the Actions one, which only ever proposes majors: read it, and label it if fine.

## Briefing a local subagent

Point it at `CLAUDE.md`, then the issue. Name the base commit and what landed
that it must respect, and the siblings with the files they edit. Tell it to
invoke `ci-merge`, write each test first and watch it fail (a bug's first
commit reproduces it, `docs/testing.md`), check the issue's blockers itself,
never merge, take a heavy build only when a slot is free and with half the
cores, and put its issue number in every scratch filename (the scratchpad is
shared). A cloud coder gets
all of this from `cloud-brief.md`.

## Model

For writing a branch (`CLAUDE.md`, *Which model writes a branch*): Sonnet for
what is purely front-end (the web app, or the desktop app's egui) and for
mechanical work (removing a feature no longer used, a translation, moving code,
a rebase); Opus for the rest, and for a mix. For work that is not a branch,
judgement takes the strongest model; when unsure, err upward.

## When a branch stops

≈3 attempts at the same failure, or a decision that is genuinely the user's (a
format change, a name people type): push what exists, mark it draft, write why
in the description and an issue comment, and take the next work. The branch
stops, never the batch. Human checks and open judgement calls never hold a
merge (`CLAUDE.md`, *Working unattended*).

## Learn from the batch: every lesson ends as a change

Each lesson ends as one of: **fixed** (a pull request removing the cause),
**enforced** (a check that makes it impossible), **documented** (this skill,
`ci-merge`, `cloud-brief.md`, `CLAUDE.md`), or **tracked** (an issue with the
evidence and a proposed fix). Mid-batch, fix what blocks or bites twice; a
Markdown-only fix lands in minutes, and one that changes a rule in `CLAUDE.md`
is left open for the user. A coder's "follow-up, not in this PR" is folded
into a branch still in flight when it fits there, filed as soon as you read it
when it carries `CLAUDE.md`'s signal, and dropped otherwise.

At the end, before reporting: list every trap, hand-back and surprise, and map
each one. Leave a *Context update* comment on the next unstarted issues whose
ground moved, and re-scope or close any the batch made moot (never a
`planning` one's scope).

## Finishing a batch

The batch is done when the main checkout runs what merged. Once nothing is
compiling:

1. Remove the worktree and local branch of every pull request whose `gh pr view
   N --json state` is `MERGED` — not its exit code, which is 0 for an open one,
   and not `git branch --merged`, which misses squashes — and of any branch with
   no commits beyond `main` and no agent standing in it. Unmerged work is only
   ever deleted by the user.
2. `git pull --ff-only` on `main`.
3. `make release`, then `cargo build --release --manifest-path app/Cargo.toml`,
   one after the other, launching nothing. A connected MCP client keeps the old
   server until it reconnects: say so.

Nothing deploys on merge. The web app is updated by hand from `main`, once, at
the end of a batch, when the operator asks, with `deploy/update` (`docs/web.md`,
*Updating*). The
desktop app, CLI and MCP server are built from source; a `v*` tag makes
`release.yml` build the CLI and MCP server, which is welcome, never required.

## Reporting back

The user does not read the transcript. **Merging is not shipping**: the report
opens with what the operator reads before deploying (`docs/web.md`,
*Updating*): the unrun human checks, from `make checks SINCE=<batch start>`,
and every SQL migration or `schema_version` bump that merged. Then the result;
where something surprised, the surprise; and the lesson → change mapping.

Two things interrupt mid-batch, because the user can only overrule them while
it runs: a change to the user's own files outside the repo, and a decision
reversed from what the issue said.
