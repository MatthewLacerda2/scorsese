---
name: issue-batch
description: Run a set of issues from board to merged — how many branches at once, which ones can safely run together, worktrees, cloud coders, re-reading the board, turning lessons into changes, and the cleanup and local rebuild that finish a batch. Use when starting work on one or more issues, when deciding what to start next, or when told to "do the issues".
---

# Working a batch of issues

The user rarely has one issue. An idea becomes several, and more appear as coding
starts. This is how a set of them gets worked without the batch costing more than
the work.

**The premise: one person, on their own machine or in a cloud session.** That is
a current fact, not a decided invariant; the day there are other contributors it
is up for review. Several rules here are shaped by it, and it is written down so
nobody re-derives the industry default of many contributors on many cold
machines and proposes the tooling that goes with it. **The machine's specs are
deliberately not written down**: the operator works on more than one, and a core
count or a free-space figure in a doc goes stale while still reading as fact.
Check the machine you are on before a heavy build (*Each branch gets its own
worktree* has how).

## The queue is the bottleneck, pipelined

Coding parallelises. **Merging does not** — Rust is compiled, so merges are
serialized, and the queue is the bottleneck.

Every branch that is not first pays a rebase for each merge ahead of it. Over
shared code that is **N(N−1)/2 rebases**: two branches cost one, three cost
three, four cost six. A rebase buys no correctness.

So what matters is that the queue never idles through a ten-minute CI run and
nothing rebases twice, not how many branches are being written. On this
machine alone that is **one in the merge queue, one being written**; with
cloud coders it is many more being written (*How many agents at once*, below).

**The merge queue is `make queue WATCH=1`** (`ci-merge` has the detail): it
rebases, pushes, waits for CI on the new head, asks `make mergeable` and merges,
one at a time — so the session running the batch is not the one sitting
through each run. It takes every ready pull request labelled **`queue`** as it
appears — fewest conflicts with the others first, then label priority, then
age, Dependabot last (`ci-merge` has why) — so feeding it is one label per pull
request once you have read it, and the batch's merge order lives on GitHub
rather than in a list a crash can lose. A hand-back (conflict, red, no run)
skips that branch until its head changes and the rest carry on, so read its
summary rather than assuming everything landed. It never resolves a conflict:
a hand-back naming paths goes back to the branch's author. `make queue PRS="a
b c"` still takes a fixed list, for a one-off.

## How many agents at once

Three kinds of agent, three limits (the operator, 2026-10-11):

- **Cloud coders: about ten in flight.** A cloud session is the default home
  for writing a branch. It brings its own CPU, memory and disk, so it costs
  this machine nothing, and a slower CPU there costs the batch nothing that
  matters. Ten is not a hardware number: it paces the token spend, and it keeps
  the batch to what one orchestrator can brief and read the diffs of. It was
  five until 2026-10-11, when the operator, having watched the usage, judged
  the project lean enough for ten.
- **Local coders: at most two.** A local coder writes a branch in a worktree
  on this machine and compiles it. The queue merges one pull request per CI
  run however many are written, so a third finishes nothing sooner, and each
  one spends the disk and cores of the machine that also hosts the web app
  (*Each branch gets its own worktree* has the numbers). Use one for what
  needs this machine (*Where a branch runs*, below), or when the cloud cannot
  take the work.
- **Agents that only read and think: no limit.** Research, review, planning,
  a search across the codebase, a second opinion on a diff — anything that
  builds nothing and writes no significant data needs no worktree and no
  `target/`, so start as many as the work wants.

The orchestrator (the batch's own session, which reviews and merges) is one
more, always (*Running a mixed batch*, below).

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

A file sitting at the size cap is one of them, whatever it holds: every branch
that appends to it must first make room, and each makes it differently. On
2026-10-10 #1014 left `crates/cli/src/lib.rs` at 298 of 300 lines of code, and
#1008, #999 and #1015 each moved a different piece of dispatch out to fit
their command. When a merge leaves a shared file within a few lines of the cap,
say so in the next `Brief` of every sibling that touches it.

**Lockfiles have one owner per wave.** Everything that touches a dependency
collides in `Cargo.toml`s and `Cargo.lock` (and `app/Cargo.lock`), and a lockfile
conflict is never a seconds-long rebase: it is a regeneration, a locked check and
`cargo deny` (`ci-merge`, *Rebasing*). So while a branch that changes
dependencies broadly is in flight, it is the **only** branch in its wave that
edits them; a sibling needing a new dependency waits for it to land, or takes
its dependency through it. Dependabot's monthly lockfile pull requests are that
broad branch while they are open. In the 2026-10-03/04 batch two were resolved
by hand: #747 added `strsim` to `app/Cargo.lock` while Dependabot's #737
rewrote it, and #748 added `schemars` to `crates/mcp`'s manifest and the lockfile
while #747 added `base64` to the same two (rusty learned the same,
MatthewLacerda2/rusty#812).

**A format change collides across modules.** Every branch that changes
`project.json` bumps `schema_version`, and that literal sits in some ninety
files — fixtures, goldens, and tests under `web/` and `app/`. Two such branches
can still run together if each bumps **last**, to `main`'s version + 1, and the
second to land renumbers (`ci-merge`, *Rebasing*). The bump also turns on the
conditional `web` and `app` gates, so a branch with no UI in it still needs Bun
and the app's libraries to prove itself. The server's SQL migrations number the
same way: two branches adding `0012_` rebase without a conflict and fail
`crates/server/tests/migrations.rs` — the second renumbers. **Hand out a
number only in landing order**: on 2026-10-16 #1406 and #1412 were each told a
migration number while both were queued, and #1412's `0036` failed the
no-gaps test until #1406's `0035` had landed. Renumber the next one in line,
let it land, then the one after.

**Reference images collide in binary.** Every branch that re-blesses an app
snapshot or a golden has a conflict no rebase can merge with any other branch
that re-blesses the same picture: it has to be re-blessed again on the new
`main`. On 2026-10-16 three app branches (#1375 palette, #1387 file tiles,
#1383 one clock) all moved `a_whole_edit.png`; each landed only after a coder
rebased and re-blessed it in turn. Sequence such branches in one lane, or plan
the re-bless as a step of the second and third.

**A new gate breaks the branches already in line.** A pull request that adds
a check (a lint rule, a dead-code gate, a clippy ban) proves the tree clean on
*its* `main`; branches written meanwhile meet it only after the queue rebases
them, as a red run. On 2026-10-16 #1368's shebang rule, #1415's knip and
#1396's `env::var` ban each failed a queued sibling (#1362, #1411, …) that way.
When such a pull request is next to merge, run its check over the queued
branches' heads and fix them before their turn; a coder briefed before the gate
existed cannot know it.

## Group the work before splitting it

**Split by responsibility, not by parallelism.** If a parent's sub-issues all
touch the same type, they are **one branch**, not one each.

Splitting an issue so several agents can run at once optimises the half that was
never scarce, and manufactures collisions: four sub-issues each adding a variant
to one enum is four rebases and four CI cycles for one
coherent change.

Sub-issues are for work that is genuinely separable *in the code* — not for work
that is merely listable.

## Each branch gets its own worktree

One checkout per branch, never two branches taking turns in one. A shared
checkout mixes another issue's edits into `make gates` and thrashes `target/`.

**Never set `CARGO_TARGET_DIR`.** Cargo's default already gives every worktree
its own `target/`; an override makes worktrees overwrite each other's artifacts
and produces a false green. `make gates` refuses one that points outside the
worktree.

What worktrees may share is sccache's compile cache, which is keyed by a hash
of each compile's inputs rather than by package, so nothing one worktree does
overwrites another's. `make setup` turns it on for every worktree under
`.claude/worktrees/` when `sccache` is installed (`cargo install --locked
sccache`, then `make setup` again), and writes nothing when it is not (#1475).

**Cores and memory bound the builds, and the machine decides the numbers.** The
operator works on more than one machine, so no limit is written down here as a
figure — measure the one you are on. The method (measured on rusty,
MatthewLacerda2/rusty#547; the rules carried over, the numbers did not):

- **A heavy build** is a cold build of a fresh `target/` or a `make gates` run.
  An incremental rebuild after an edit is not one. One cold build already fills
  every core, so two side by side take as long as the same two back to back: a
  second heavy-build slot exists so one agent is not queued behind another's
  build, never for throughput, and a third buys nothing.
- **Give each heavy build half the cores** (`CARGO_BUILD_JOBS` = half of
  `nproc`, or `sysctl -n hw.ncpu` on macOS) whenever another may overlap it. On
  rusty two capped builds finished as fast as two uncapped ones and peaked about
  2 GB lower. A build alone runs uncapped.
- **Check before a heavy build**, on this machine, today: free memory and free
  disk (`df -h`; `free -g` on Linux, `vm_stat` or Activity Monitor on macOS).
  If you do not know what one cold build costs here, measure one first and keep
  a margin. Running out still shows up as a link-step failure or a killed rustc,
  not as "no disk" — the machine's fault, not the branch's (`ci-merge`).
- **On the machine that hosts the service** (#527, #532: the web app in Docker
  Compose behind a Cloudflare Tunnel), the cores are shared with other people:
  their renders and generations queue on it, and a heavy build competes with a
  paying user's render. Leave it headroom.
- **A warm `target/` is the fast path; `sccache` only shortens a cold one.**
  It does not turn incremental compilation off. It caches the third-party
  dependency compiles and passes the workspace crates through. Measured on
  2026-10-17 (`cargo nextest run --workspace --no-run`, `CARGO_BUILD_JOBS=4`):
  - a cold worktree took 11m17s without it and 11m20s with an empty cache;
  - with a warm cache it took 8m57s (270 of 270 cacheable compiles hit);
  - the edit loop after touching `crates/core` was unchanged (6m52s / 6m25s,
    8 crates rebuilt incrementally both times);
  - the cache is ~170 MB.

  So it saves about two minutes per new worktree on one machine, costs
  nothing, and does nothing for cloud coders, which don't share the cache.
  Rusty uses it the same way (MatthewLacerda2/rusty#492); #1475 ports its `make setup`.
- **When disk is what binds** (a laptop: 26 GB free on 2026-09-30, and a
  worktree's `target/` 8–14 GB, most of it `debug/incremental`), a
  git-excluded `.cargo/config.toml` in each worktree with `debug =
  "line-tables-only"` for the dev and test profiles and `[build] incremental =
  false` brought a full build to ~4.6 GB. No test outcome changes; the warm
  incremental rebuild is the price, so it is for a machine short of disk, never
  the default.

**Remove a worktree the moment its branch merges.** Each carries a full
`target/` — up to ~40 GB after `make gates` (#900's, 2026-10-08). Disposal is what keeps disk from becoming the
overnight failure, and it fails as a confusing build error rather than as "no
disk".

## Cloud sessions are extra coders, not extra merges

The build limits above are the local machine's, not the workflow's. A **cloud
session** brings its own CPU, memory and disk, so it lifts the cap on how many
branches can be *written* at once — without consuming the operator's hardware.
That makes it the default place to write a branch (*How many agents at once*).
It does nothing for merging — that queue is still one CI run at a time — so
more cloud coders help while writing is the bottleneck (hours-long foundation
branches), never once the queue is.

**Where a branch runs:**

- **Local:** the batch's own session (the orchestrator); every merge; anything
  that needs files only the operator's machine has (a real `.scor` project, real
  media, provider keys — `make live-check` spends money and never runs in a
  cloud session); anything whose proof needs a real display, a real GPU or real
  speakers; anything macOS-specific.
- **Cloud:** a branch whose proof is `make gates` **and** that edits no types or
  files another in-flight branch edits. The collision list above still decides
  that; cloud removes the build-slot limit, not the rebase cost. A cloud
  container is Linux, so it also runs the pixel gate that `make gates` skips on
  macOS — with whatever ffmpeg the container installs, so CI stays the authority
  (`docs/golden-renders.md`). That makes it the natural home for a branch that
  **adds a golden fixture**, provided the container is x86_64: a reference is
  only trustworthy blessed on CI's platform, **x86_64 Linux**. Linux alone is
  not enough. On 2026-09-30 an arm64 `ubuntu:24.04` on the Mac reproduced every
  existing fixture but `grade_*` and `vhs`, and still blessed #583's new
  `dashes` fixture wrong (CI: ssim 0.9419, needs ≥ 0.95) — thin anti-aliased
  strokes take different float paths. Off an x86_64 Linux machine, bless in an
  `--platform linux/amd64` container, never a native arm64 one.
- **Either, if it has docker:** a branch in `crates/server` or `deploy/` is
  proven against a real Postgres and `docker compose`. A machine that must not
  run them (the operator said so of the Mac on 2026-09-30) is the wrong place
  for it. A cloud container has both: its session hook starts a native
  Postgres, and `dockerd` can be started (`cloud-brief.md`, *The container*).

**How many:** about **ten cloud coders in flight**, beside at most two local
ones (*How many agents at once* has why). Ten is a suggestion, not a rule: the
orchestrator sizes the batch from the sessions actually running and from the
tech stack. Files that aren't compiled collide too: two branches in
`web/`'s React components rebase against each other like any Rust enum. Every
branch behind another still pays a rebase per merge ahead of it, and the queue
drains about one pull request per CI run, so past the point where the queue is
the bottleneck, more coders only lengthen it.

**Launching one — and proving it is one.** Not with the `Agent` tool's
`isolation: "remote"`: in a local session that silently falls back to a local
worktree (seen on rusty, MatthewLacerda2/rusty#504). What works is a **one-off
cloud routine** — the `RemoteTrigger` tool (the `schedule` skill has the body
shape) with `run_once_at` a minute or two out, this repository as its source,
and the short fixed prompt below (the brief itself is an issue comment, #820).
Compute that time, never type it: `date -u -d '+2 min' +%Y-%m-%dT%H:%M:00Z` on
Linux (`date -u -v+2M …` on macOS). On rusty a hand-typed time fired 45 minutes
late, and the API refuses a time already past. **Creating routines is rate-limited** (2026-10-16): about four
`create` calls in a burst, then `429 … Try again in 15s`. Send them in fours,
wait the seconds it names, and give each a `run_once_at` far enough out that
the wait does not make it already past. Set its **model** on purpose: the `schedule`
skill's example body defaults to a Sonnet, and writing a feature branch is
judgement work (*Model, as a hint* below). Its example
`allowed_tools` lists only shell and file tools, so check it leaves room for
what the brief uses (ToolSearch, the GitHub MCP tools). Two traps
decide whether it is really remote:

- **Pick the `anthropic_cloud` environment, never a `bridge` one.** The
  environment list can include a *bridge* to the operator's own machine; a
  routine on it runs **on that machine**, building on its cores and disk. This
  is the likely cause of rusty's "cloud" sessions that ran locally. (Scorsese's
  account listed only `Default`, `anthropic_cloud`, on 2026-09-30 — re-read
  the list rather than trust that.)
- The brief's **first instruction** (step 0 below) prints `$CLAUDE_CODE_REMOTE`,
  which Claude Code sets to `true` in a cloud session. If it is not `true`, or
  the path is under the operator's home, the session **stops before touching
  anything** and reports that it is local.

Then check from here: `list_runs` / `get_run_log` on the routine show the
session's first tool result (`CLAUDE_CODE_REMOTE=true`, a `/home/user/…` path),
and no new worktree or local `cargo`/`rustc` process appeared for that branch.
The cloud session has GitHub MCP tools (`create_pull_request`,
`add_issue_comment`), so it can open and update its own pull request.

**The pull request is the report.** A cloud session cannot message the
orchestrator, and nothing notifies the orchestrator when it finishes. Brief it
to open a draft on its first commit, push often (a dead container takes its
uncommitted work with it), write decisions and hand-backs into the description
and an issue comment (never a push notification to the operator, who checks in
from the terminal), and mark the PR ready when its gates are green. **Watch the
PR, not the agent**: the queue's watch reports each PR turning ready as a
`watch:` line (*Running a mixed batch*, below); the orchestrator reads it then
and labels it `queue`, and the same watch takes it from there.

**Ready means finished.** The batch merges a ready PR the moment its CI is green,
so a PR is never readied just to make CI run. On rusty a cloud session readied
its PR to get CI on a temporary 200-round test loop, and it merged with the loop
still in (MatthewLacerda2/rusty#561). Here CI runs only on ready pull requests
and `ci.yml` has no manual trigger, so the proof run for a draft is the gate
itself, run in the session — a cloud container has the room for `make gates`.

**Once ready, the branch is the merge queue's.** Its coder does not push to it
again: a red run comes back to the orchestrator as a hand-back, and the
orchestrator fixes it or re-briefs a coder (`cloud-brief.md` says the same). A late push costs a full CI round, because
`make queue` rightly refuses to merge a head it did not watch; anything found
afterwards (a mutation survivor, a missing test) goes in an issue comment, for a
follow-up PR off `main` (MatthewLacerda2/rusty#583).

**A cloud session is never woken by its own background work.** A routine session
that starts a build in the background and ends its turn to wait sits idle
forever — on rusty one did exactly that for an hour, gates half-run, PR still a
draft (MatthewLacerda2/rusty#519). Brief every cloud session to run builds and
gates in the **foreground** (long timeouts, split across calls). It never waits
on CI: it marks its pull request ready and ends, and the merge queue waits for
the run; a red run comes back to the orchestrator as a hand-back. Anything else
it must wait for (a measurement workflow, say) is polled in the foreground too.
From here, `worker_status: idle` on `list_runs` with a draft PR is the stall's
signature, **but read the run's log before calling it one**: on 2026-10-05
#775's coder sat `idle` between notifications from a background mutation sweep
and its own Monitor, and each one woke it, so the background wait worked there.
Idle with a background task still running, and the log's last turn saying what
it waits for, is waiting; idle with nothing pending is the stall, and the fix
is a fresh routine briefed to finish the pushed branch.

**The container's disk is finite too**, and **what it costs** is a cold build of
the whole dependency tree, with no local compile cache to help — so choose it on
purpose, and have it run the full `make gates` once before readying (after a
rebase over unrelated merges, `cloud-brief.md` has the lighter check).

## Running a mixed batch: one orchestrator, cloud coders, two local slots

This is the shape that ran rusty's 2026-09-30 batch (≈40 pull requests merged in
one day, MatthewLacerda2/rusty#567). Each role does what only it can do.

**The orchestrator (the batch's session, local)** writes no feature code. It
picks work, briefs cloud coders, reviews and merges. Its loop per ready pull
request:

1. Read the description, **and the diff** — a green gate does not read code (on
   2026-09-30 an agent's `unreachable!` inside a `Display` impl was caught here,
   not by CI). Check the decisions against the issue, and note any human-only
   checks (a window, speakers, taste) as a checklist; don't hold for them.
2. **Label it `queue`**, with `make queue WATCH=1` running in the background
   for rebase → push → wait → merge; don't hand-roll that loop. **It is the
   orchestrator's one background watch** (#819): run `make queue WATCH=1
   SINCE=<the batch's start, ISO 8601> > <scratch>/queue-watch.log 2>&1` as a
   background command, and a `Monitor` on `tail -F <that log> | grep
   --line-buffered '^watch: '`. **Never the watch itself under `Monitor`**: a
   Monitor is killed at thirty minutes and kills its command with it, and the
   watch runs seventy. **Never behind `| tail`** either: on 2026-10-06 a watch
   started as `make queue WATCH=1 | tail -40` took nothing for 44 minutes while
   a PR it should have merged waited, and nothing it printed could be read
   until it was killed; restarted with its output in a file, it took the PR at
   once. Every `watch:` line that arrives is an event to act
   on — a PR `opened as a draft`/`opened as ready`, `turned ready` (read it,
   label it), `back to draft`, `closed, not merged by this watch`, and each one
   the queue finished: `merged`, `handed back` (with the reason), `unfinished`,
   `unreachable`. The first listing after a launch says nothing, and a listing
   GitHub would not give is skipped rather than read as every PR changing. It
   is bounded to the harness's two-hour cap; its **last line is `watch:
   resume: <command>`** — start exactly that command again the same way (a
   background command into the log, the Monitor re-armed on it; the Monitor
   also needs re-arming each time it expires). Exit status 4 means something was still unfinished, 3
   that GitHub stopped answering; the resume line is the answer to both. Exit
   status 5 means another queue already holds this repository's lock (#1346):
   its pid is in the line, and nothing was attempted — never start a second. It merges only on `make mergeable`'s verdict. It does
   **not** compile the rebased tree before pushing, and a clean textual rebase
   still breaks when a merge ahead changed a signature this branch uses — on
   rusty a removed argument cost a ten-minute CI round to find, and a rebase
   pushed a file past the size cap. When the merges ahead touched the same
   crates, rebase in the branch's worktree yourself, `cargo check --workspace
   --all-targets --locked` (plus `--manifest-path app/Cargo.toml` when the
   branch touches `app/`) and `make size`, and push: the queue then finds
   nothing to rebase and only waits and merges. The check is a heavy build on
   a cold target, so it takes a build slot like any other. A **cloud-written**
   branch has no local worktree, and a cold one here spends the disk the cloud
   was meant to save: check it locally only when a merge ahead changed a
   signature it calls, and otherwise let CI answer.
3. A hand-back is the queue's whole report: fix a conflict or a red run on the
   branch (or brief its coder to). The push is the re-queue: the watch skips a
   handed-back branch until its head changes, then takes it again on its own.
4. After merging: remove the worktree and its `target/`, re-read the board, and
   start the next piece of work. There is no mutation report to read now: pull
   requests carry none (#651), so a branch that wanted one asked for it with
   `make mutants-remote` before it was readied, and its survivors were handled
   then (`ci-merge` has the triage).

**Cloud coders write the branches.** Pick an issue for the cloud when its proof
is `make gates` and **no in-flight branch edits the same types or files**.
Parallelism comes from spreading across areas (`zimmer`, `providers`, `mcp`,
`server`, `web/`, `app/`, CI) at once, not from stacking work in one. A crate
is too coarse a unit here: nearly every editing feature touches both `core`
and `compositor`, so sort those by the collision rule above — the shared
appends (validation errors, the per-layer compositing path, the format doc,
the schema bump) — rather than by crate, or the batch runs one at a time.

**The local slots (at most two coders) take what only the operator's machine
can do:** measurements
that decide something (a before/after table in the pull request), fixes that
need a real display, GPU or speakers, anything needing the operator's files or
keys, and the orchestrator's own compile checks. Don't let the orchestrator
compile while a local agent is timing builds; it skews the numbers.

**The brief is a comment on the issue; the prompt only points at it** (#820,
ported from MatthewLacerda2/rusty#910). `RemoteTrigger create` echoes a prompt
back about four times (prompt, job config, session request, payload), so every
line in it is paid for four times in the orchestrator's context: on 2026-10-05/06
twelve launches with 1–3 KB briefs were a large share of it, and on rusty the
same reached 85% after ~45 launches (MatthewLacerda2/rusty#579). A brief that
lives only in a routine's prompt is also invisible to the next orchestrator, to
the coder's reviewer and to anyone resuming after a crash. So the issue-specific
part goes on the issue as a comment whose **first line is `Brief`**, where it is
durable and sits beside the issue it qualifies. The standing rules (foreground
builds, the blocker check, ready means finished, the container, blessing, the
repo's traps, the mutation signal, the PR protocol, "do not merge") live in
[`cloud-brief.md`](cloud-brief.md), versioned with this skill; a change to them
goes in that file, once. The comment:

```text
Brief

Branch: `N-short-slug`.
Builds on: <what merged that it must build on; line numbers in issues go stale
within hours>.
Siblings in flight: <branch/issue → the files it edits; stay out of them>.
Decisions: <anything already settled, or "none">.
```

The routine prompt is then the same few lines for every coder:

```text
STEP 0 — before anything else, prove you are a cloud session. Run
`echo "CLAUDE_CODE_REMOTE=$CLAUDE_CODE_REMOTE"; pwd; uname -a`. If it is not
exactly `true`, or the path is under the operator's home, STOP: touch nothing,
end with "LOCAL — aborted" and that output.

Then read `.claude/skills/issue-batch/cloud-brief.md` and follow it.
Issue(s): #N. Your brief is the newest comment on it that starts with `Brief`.
```

A re-brief (a rebase request, a new sibling to avoid) is a **new `Brief`
comment and a fresh routine**, never an edit to an old prompt.

### What the orchestrator keeps, and where (2026-10-03)

- **GitHub is the batch's state.** Open PRs, their draft/ready state and comments, plus `RemoteTrigger list` for the routines, are enough to resume a batch from nothing. The orchestrator's own notes (a queue list, a routine table) are conveniences. Never keep anything **only** in `/tmp`: the scratchpad lives there, and a reboot wipes it. On 2026-10-02 a machine crash mid-batch took the merge queue's list and the routine table with it, and the batch resumed from GitHub alone.
- **One background watch, not two.** The queue's watch reports the board as well as its merges (#819), so there is no separate `gh pr list` poller to keep alive. On 2026-10-05/06 that poller expired every thirty minutes, was re-armed about eight times, and once, after a network blip, announced every open PR as new; the queue's watch was restarted four more times beside it. The supervisor that would relaunch the watch on its own (MatthewLacerda2/rusty#908) is not here yet: re-arming is still the orchestrator's, once per resume line.
- **Stop a watch by its PID, never `pkill -f merge-queue.py`.** Every repository the operator batches runs the same script name, and on 2026-10-05 a rusty session's `pkill -f "merge-queue.py --watch"` killed scorsese's watch mid-wait. Nothing was lost (it was only waiting on CI, and a re-run resumes), but the same pattern mid-push would have been a hand-back to untangle. Hold the PID of the watch you started (the harness's background task) and stop that.
- **Never wait on another process by `pgrep -f <text>`.** The waiting shell's own command line contains the text, so it waits on itself forever (about 25 minutes lost on 2026-10-02). Wait on a PID you hold. With `make queue WATCH=1` (#690) there is nothing to chain: one watch takes everything labelled `queue`, and the merge order is on GitHub, not in a list.
- **Watch each PR's head commit, not only its draft/ready state.** A rebased PR stays *ready* the whole time, so a watcher keyed on state never sees the push. The queue's watch keys on the head for exactly this reason; your own reading of a re-pushed PR still waits for the coder's PR comment ("rebased, gates green"). If the push should *not* be retaken yet, take the `queue` label off first.
- **The queue lands the broad PR last itself (#738).** A PR that adds an asset kind or a tool touches every registry, so whichever lands first sends every sibling touching those lists back with a conflict. On 2026-10-03, #677 landing first handed back #676, #686 and #673, each rebased one after another (an hour apiece). The watch now takes the PR that textually conflicts with the fewest others in line first, so the small appenders land and the broad PR takes the one hand-back. Nothing to remember; label all of them `queue` together and the order follows. #691 removes most of these conflicts at the source.

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

**Re-read the whole board, not your list of it.** Coders file what they find
mid-batch, and the operator files from other machines. A batch that tracks only
the issues it started stops early and misreports what is left: on 2026-10-03
the operator filed #720, #721 and #722 at 23:51, mid-batch, and the
orchestrator learned of them only when the operator asked. Rusty's batch made
the same mistake on 2026-10-02. Count what is left from `gh issue list`, never
from memory. An issue filed mid-batch is startable at once unless it carries a
stage label, and it can outrank the work in flight: when a new label-priority
leader appears, it is the one started next.

**Dependabot's pull requests are on the board too, and need nothing from you.**
`.github/dependabot.yml` opens at most four a month (Actions, the root and
`app/` Cargo workspaces, `web/`), grouped, semver-compatible, majors ignored.
They have no issue of their own and need none (a pull request needs an issue
only when its work needed planning; #721 decided these),
and all but the Actions one label themselves `queue`: the watch takes them at the **lowest priority**, after
every other pull request, and asks `@dependabot rebase` rather than ever
force-pushing their branch (`ci-merge` has the detail). Read one only when the
queue hands it back. A major upgrade is never a bot pull request; it is an
issue like any other. **The Actions one is the exception, and waits for a
person**: Actions are pinned at majors, so every bump it proposes is a major,
and the workflows it touches outside pull-request CI cannot be proven by its
run (#734, the first, moved three actions 4 -> 7). Read it, and label it
`queue` if it is fine.

But re-reading is not a licence to start everything: **start the next one
inside the limits of *How many agents at once***, and keep a local slot for
whatever is furthest along. Priority orders what gets
merged. An unblocked issue left unstarted is not wasted capacity; it is a rebase
not yet paid for.

**A stage label is the only absolute stop.** `planning` and `human` mean
*not yet* (`idea` was retired into `planning`, #501), and no amount of the issue looking ready overrides that. Everything
else is startable the moment it exists, including an issue filed a minute ago.

## Briefing a subagent

Point it at `CLAUDE.md` first, then the issue — issues here are written to be
read cold. Beyond that:

- Name the **base commit** and what has landed recently that it must respect.
- Name the **siblings** and which files they are touching.
- Tell it to invoke the **`ci-merge` skill** rather than restating that protocol.
- Tell it to **check the issue's blockers itself** before writing code: a
  blocker written only in the issue's prose is invisible to the orchestrator,
  which reads GitHub's recorded relationships (`cloud-brief.md` has the step).
- For a **cloud** session: launched as a one-off routine on the
  `anthropic_cloud` environment, step 0's `CLAUDE_CODE_REMOTE` check comes
  first, and the pull request is its only way to report (see above). Its
  standing rules — foreground builds, the blocker check, never pushing a ready
  branch, numbers taken last, the PR's *Gates* line, do not merge — are
  [`cloud-brief.md`](cloud-brief.md)'s; point at it, never restate it.
- For a **local** subagent, the same rules apply, from `ci-merge` and this
  skill: tell it **not** to merge (merging is serialized and belongs to the
  session running the batch), not to start a heavy build while the machine's
  heavy-build slots are taken, to give it half the cores (`CARGO_BUILD_JOBS`)
  while another may overlap it, and not to run `make mutants` while siblings
  are compiling.
- **Scratch filenames must carry the issue number.** The scratchpad is shared
  between sibling agents; a collision has already swapped one pull request's
  description for another's.

## Model, as a hint

**For writing a branch, the area decides** (the maintainer, 2026-10-14;
CLAUDE.md, *Agent velocity*):

- **Sonnet** when the branch is only front-end and changes what the user sees
  on screen, but never how the app uses the API (`web/` components, copy,
  layout, styling), or when it is only the desktop app's egui (`app/`).
- **Opus** for DevOps (CI, `deploy/`, Terraform), networking, and back-end
  (`crates/`, the server, anything that changes a request or a response).

A branch that mixes the two takes Opus, or is split. **A Sonnet's pull
request carries the `minor` label** (the maintainer, 2026-10-15), so which
model wrote a change is visible from the pull-request list. The coder adds it
when it opens the pull request (`cloud-brief.md`); whoever reviews it checks
it is there before adding `queue`, and adds it if not. The rest of this
section is for everything that is not writing a branch.

Judgement work — design, implementation, triage — wants the strongest model. A
rebase, a module-list conflict, an attribute moved between files does not. Most
sessions on a branch are the second kind, and paying top rate for them is where
a night's budget quietly goes. A hint because the line is not crisp — a
"mechanical" rebase that turns out to need two authors' prose reconciled is not
mechanical — so whoever spawns the work calls it, and gets it wrong upwards.

A cheaper model also follows the standing rules less closely, so read its
*Gates* line before labelling. On 2026-10-10 #1043's Sonnet coder opened its
pull request ready, having run `fmt`, `size`, `clippy` and one crate's tests
rather than `make gates`. It was a string change and CI was green, but the
claim a ready pull request makes was not one it had checked.

## When to hand back to the user

- ≈3 attempts at the same failure.
- A decision that is genuinely theirs: a format change, a name people will type,
  anything a `planning` label would have carried.
- **Save, ask, move on.** Push what exists (a dead session takes uncommitted
  work with it), mark the pull request **draft**, write what is needed and why
  in the description and an issue comment, then **take the next startable
  work**. The branch stops, never the batch (the operator on rusty, 2026-10-02:
  "the work is saved but the progress doesn't stop"). Do not thrash on the
  stopped branch.

When working unattended, prefer leaving a comment on the issue and continuing
over stalling the night on a question.

**A human check is never a merge hold.** Some proof only a person can give: a
real window, real speakers, taste. When a green pull request carries such a
check, merge it anyway, and put the checklist in the PR description and an issue
comment for the user to run later. If it turns out broken, that is a bug to file
and fix, not a reason the batch waited. The same goes for a judgement call the
issue left open: take the default the issue, CLAUDE.md or Filmora 9 (for taste)
points to, write down what you chose and why, and keep going. The batch exists
so the user does not have to be there. (On rusty, holding three PRs for a smoke
test and an ear check stalled the queue for hours, and every one was fine —
MatthewLacerda2/rusty#566.)

**Merging is not shipping.** Paying users see a change only when the operator
runs the deploy's update step (`docs/web.md`, *Updating*), and what that step
does to their data only goes forward: the server's SQL migrations, and a
`schema_version` bump rewriting every stored project. So the report lists,
before anything else, every unrun human check and every such migration in what
merged — that list is what the operator reads before updating the service. The
human checks come from `make checks SINCE=<when the batch started>` (#821),
never gathered by hand: it reads every merged description's human-check section
and leaves out what the user has already ticked.

## Learn from the batch: every lesson ends as a change

A batch is trial by fire: it finds the traps, gaps and bugs in the tooling and
in scorsese faster than anything else. A lesson that only reaches the chat is
lost, because the next session starts without it. So every lesson ends as
exactly one of:

- **fixed**: a pull request that removes the cause;
- **enforced**: a lint, gate or test that makes it impossible to repeat;
- **documented**: this skill, `ci-merge`, `cloud-brief.md` or
  CLAUDE.md, for what the repo can't change (a platform behaviour, a procedure);
- **tracked**: an issue with the evidence (run ids, PR numbers, measurements) and
  a proposed fix, never just the symptom.

**Mid-batch, fix what blocks or bites twice.** When a lesson costs the running
batch time — a tooling bug that stalls the queue, a brief that makes every agent
re-solve the same trap — fix it now. It is infrastructure, so it outranks
whatever was next. A skill or doc edit is Markdown-only: its CI run compiles
nothing (except for a page the code reads, which `ci-paths.py` lists), so the
queue takes it in a minute or two (`ci-merge` has how). One that changes a **rule in `CLAUDE.md`** rather than recording a step
is a change to scorsese's conventions: open it and leave it for the user
(*Take the initiative*). On rusty's
2026-09-30 batch, five tooling and skill fixes landed within the hour their
lesson appeared (MatthewLacerda2/rusty#577).

**Otherwise, file it and keep going.** The strongest issues come from doing the
work. File an agent's "follow-up, not in this PR" notes and a PR's "decisions to
check" the moment you read them, not at the end.

**At the end, run a retro before reporting back:**

1. List every trap, gap, surprise, workaround and hand-back from the batch: your
   own notes, the decisions and untested parts each PR's description names, the issues the
   agents filed.
2. Map each one to fixed, enforced, documented or tracked. Anything unmapped gets
   an issue now.
3. **Refresh the unstarted issues whose ground moved.** A batch invalidates line
   references and assumptions fast, so add a short *Context update* comment to
   the issues next in line: what merged that they build on, and what to rebase
   over. Re-scope or close any that the batch made moot. Never touch a
   `planning` issue's scope.
4. Put the mapping in the report, so the user sees each lesson turned into a
   change rather than only described.

## Finishing a batch

The batch is not done when the last branch merges — it is done when the main
checkout runs what was merged. Last, once nothing is compiling:

1. **Clean up what is finished.** Remove the worktree and delete the local
   branch of every pull request that is `MERGED` — ask `gh pr view N --json
   state`, never its exit code, which is 0 for an open one too. `git branch
   --merged` cannot see a squash merge, so it is not the test. A branch with
   **no commits beyond `main`** (`git rev-list --count origin/main..BRANCH` is
   0) goes too, once its worktree has nothing uncommitted and no agent is still
   standing in it. Anything else stays: unmerged work is only ever deleted by
   the user.
2. **Bring the main checkout up to date** — `git pull --ff-only` on `main`.
3. **Rebuild the local tools, and launch nothing.** `make release` builds the
   workspace in release mode, `scorsese` and `scorsese-mcp` among it — the MCP server a local client is pointed at
   is `target/release/scorsese-mcp`, so until this runs every session drives the
   old code. Then `cargo build --release --manifest-path app/Cargo.toml` for the
   desktop app. One after the other, never beside a sibling's build. A client
   already connected keeps the old server process until it reconnects; say so
   in the report.

## Reporting back

The user is not reading the transcript of a batch. They take long — often hours,
usually overnight — and the transcript is, if anything, notes for Claude itself.

**The unrun human checks open the report**, pasted from `make checks
SINCE=…` (see *Merging is not shipping*).

**When things go well, say what the result was.** When things did not go as one
would expect, say what the surprise was. That does not necessarily mean things
went badly: we write it down because the more we can predict, the better we
improve. Include the retro's lesson → change mapping.

Two things still interrupt, because they are the ones the user would want to
overrule and overruling is only possible while the batch is still running: **a
change to the user's own files** outside the repo, and **a decision reversed** —
where the issue said one thing and the branch did another.
