---
name: issue-batch
description: Run a set of issues from board to merged — how many branches at once, which ones can safely run together, worktrees, cloud coders, re-reading the board, turning lessons into changes, and the cleanup and local rebuild that finish a batch. Use when starting work on one or more issues, when deciding what to start next, or when told to "do the issues".
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

**The merge queue is `make queue PRS="a b c"`** (`ci-merge` has the detail): it
rebases, pushes, waits for CI on the new head, asks `make mergeable` and merges,
one at a time, in the order given — so the session running the batch is not the
one sitting through each run. Hand it the ready pull requests in label-priority
order; a hand-back (conflict, red, no run) skips that entry and the rest carry
on, so read its summary rather than assuming the whole list landed. It never
resolves a conflict: a hand-back naming paths goes back to the branch's author.

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

**A format change collides across modules.** Every branch that changes
`project.json` bumps `schema_version`, and that literal sits in some sixty
files — fixtures, goldens, and tests under `web/` and `app/`. Two such branches
can still run together if each bumps **last**, to `main`'s version + 1, and the
second to land renumbers (`ci-merge`, *Rebasing*). The bump also turns on the
conditional `web` and `app` gates, so a branch with no UI in it still needs Bun
and the app's libraries to prove itself. The server's SQL migrations number the
same way: two branches adding `0012_` rebase without a conflict and fail
`crates/server/tests/migrations.rs` — the second renumbers.

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
- **On the machine that hosts the service**, a heavy build also competes with
  users' renders (CLAUDE.md); leave it headroom.
- **When disk is what binds** (a laptop: 26 GB free on 2026-09-30, and a
  worktree's `target/` 8–14 GB, most of it `debug/incremental`), a
  git-excluded `.cargo/config.toml` in each worktree with `debug =
  "line-tables-only"` for the dev and test profiles and `[build] incremental =
  false` brought a full build to ~4.6 GB. No test outcome changes; the warm
  incremental rebuild is the price, so it is for a machine short of disk, never
  the default.

**Remove a worktree the moment its branch merges.** Each carries a full
`target/` — 8–17 GB apiece. Disposal is what keeps disk from becoming the
overnight failure, and it fails as a confusing build error rather than as "no
disk".

## Cloud sessions are extra coders, not extra merges

The build limits above are the local machine's, not the workflow's. A **cloud
session** brings its own CPU, memory and disk, so it lifts the cap on how many
branches can be *written* at once — without consuming the operator's hardware.
It does nothing for merging — that queue is still one CI run at a time — so
reach for cloud when writing is the bottleneck (hours-long foundation branches),
never when the queue is.

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
  for it; the cloud may be too, until `cloud-brief.md`'s container list is
  confirmed.

**How many:** two in flight stays the default shape; with cloud coders writing,
the ceiling is **four branches in flight in total**, local and cloud together,
with the local share set by the heavy-build slots above. Each one
behind another still pays a rebase per merge ahead of it, and a full queue
drains about one pull request per CI run: four finishing together leave the last
waiting most of an hour with three rebases paid. Past four, the queue is the
bottleneck and more coders only lengthen it.

**Launching one — and proving it is one.** Not with the `Agent` tool's
`isolation: "remote"`: in a local session that silently falls back to a local
worktree (seen on rusty, MatthewLacerda2/rusty#504). What works is a **one-off
cloud routine** — the `RemoteTrigger` tool (the `schedule` skill has the body
shape) with `run_once_at` a minute or two out, this repository as its source,
and the brief below as its prompt. Set its **model** on purpose: the `schedule`
skill's example body defaults to a Sonnet, and writing a feature branch is
judgement work (CLAUDE.md, *Which model does what*). Its example
`allowed_tools` lists only shell and file tools, so check it leaves room for
what the brief uses (ToolSearch, the GitHub MCP tools, `send_later`). Two traps
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
and an issue comment, and mark the PR ready when its gates are green. **Watch the
PR, not the agent**: the orchestrator polls GitHub (`gh pr view N --json
isDraft,state`, or `gh pr list --search "is:open -is:draft"`) on an interval and
queues a PR the moment it turns ready.

**Ready means finished.** The batch merges a ready PR the moment its CI is green,
so a PR is never readied just to make CI run. On rusty a cloud session readied
its PR to get CI on a temporary 200-round test loop, and it merged with the loop
still in (MatthewLacerda2/rusty#561). Here CI runs only on ready pull requests
and `ci.yml` has no manual trigger, so the proof run for a draft is the gate
itself, run in the session — a cloud container has the room for `make gates`.

**Once ready, the branch is the merge queue's.** Its coder does not push to it
again, except to fix its own red CI. A late push costs a full CI round, because
`make queue` rightly refuses to merge a head it did not watch; anything found
afterwards (a mutation survivor, a missing test) goes in an issue comment, for a
follow-up PR off `main` (MatthewLacerda2/rusty#583).

**Cloud coders share the account's usage limit.** When it runs out, every
cloud session stops at the same moment as the orchestrator, and its container's
unpushed work is lost (2026-09-30: four at once, `rate_limit: rejected
(five_hour)` in each `get_run_log`). After the reset, run `list_runs` on every
routine still in flight: `worker_status: idle` with a draft PR is the
signature. A stopped session cannot be messaged from here, so launch a fresh
routine per branch, briefed to **finish the pushed branch**: what it has, what
was lost, and what main has done since, since a rebase is usually owed by then.

**A cloud session is never woken by its own background work.** A routine session
that starts a build in the background and ends its turn to wait sits idle
forever — on rusty one did exactly that for an hour, gates half-run, PR still a
draft (MatthewLacerda2/rusty#519). Brief every cloud session to run builds and
gates in the **foreground** (long timeouts, split across calls), and to schedule
its own check-in with `send_later` (the `Claude_Code_Remote` MCP tool) before
ending a turn to wait on CI. From here, `worker_status: idle` on `list_runs` with
a draft PR is the stall's signature; the fix is a fresh routine briefed to
finish the pushed branch.

**The container's disk is finite too**, and **what it costs** is a cold build of
the whole dependency tree, with no local compile cache to help — so choose it on
purpose, and have it run the full `make gates` before readying.

## Running a mixed batch: one orchestrator, cloud coders, a local slot

This is the shape that ran rusty's 2026-09-30 batch (≈40 pull requests merged in
one day, MatthewLacerda2/rusty#567). Each role does what only it can do.

**The orchestrator (the batch's session, local)** writes no feature code. It
picks work, briefs cloud coders, reviews and merges. Its loop per ready pull
request:

1. Read the description, **and the diff** — a green gate does not read code (on
   2026-09-30 an agent's `unreachable!` inside a `Display` impl was caught here,
   not by CI). Check the decisions against the issue, and note any human-only
   checks (a window, speakers, taste) as a checklist; don't hold for them.
2. **Use `make queue PRS="N"`** for rebase → push → wait → merge; don't
   hand-roll that loop. It merges only on `make mergeable`'s verdict. It does
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
   branch (or brief its coder to), then queue it again.
4. After merging: remove the worktree and its `target/`, re-read the board, and
   start the next piece of work. Then read the merged PR's mutation comment:
   the queue merges without reading it, so survivors in code the branch wrote
   become one follow-up issue now (`ci-merge` has the triage) — the code is no
   longer in hand, and nobody else will look.

**Cloud coders write the branches.** Pick an issue for the cloud when its proof
is `make gates` and **no in-flight branch edits the same types or files**.
Parallelism comes from spreading across areas (`zimmer`, `providers`, `mcp`,
`server`, `web/`, `app/`, CI) at once, not from stacking work in one. A crate
is too coarse a unit here: nearly every editing feature touches both `core`
and `compositor`, so sort those by the collision rule above — the shared
appends (validation errors, the per-layer compositing path, the format doc,
the schema bump) — rather than by crate, or the batch runs one at a time.

**The local slot takes what only the operator's machine can do:** measurements
that decide something (a before/after table in the pull request), fixes that
need a real display, GPU or speakers, anything needing the operator's files or
keys, and the orchestrator's own compile checks. Don't let the orchestrator
compile while a local agent is timing builds; it skews the numbers.

**Every cloud brief carries** step 0 inline, then a pointer to
[`cloud-brief.md`](cloud-brief.md) — the standing rules (foreground builds,
`send_later`, ready means finished, the repo's traps, the PR protocol, "do not
merge") live there, versioned with this skill — and only what is specific to
its issue. `RemoteTrigger` echoes a prompt back several times, so standing text
copied into each one fills the orchestrator's context (on rusty it reached 85%
after ~45 launches, MatthewLacerda2/rusty#579); a change to the rules goes in
the file, once. The prompt:

```text
STEP 0 — before anything else, prove you are a cloud session. Run
`echo "CLAUDE_CODE_REMOTE=$CLAUDE_CODE_REMOTE"; pwd; uname -a`. If it is not
exactly `true`, or the path is under the operator's home, STOP: touch nothing,
end with "LOCAL — aborted" and that output.

Then read `.claude/skills/issue-batch/cloud-brief.md` and follow it.

Issue(s): #N — <one line>. Branch: `N-short-slug`.
Builds on: <what merged that it must build on; line numbers in issues go stale
within hours>.
Siblings in flight: <branch/issue → the files it edits; stay out of them>.
Decisions: <anything already settled, or "none">.
```

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
- For a **cloud** session: launched as a one-off routine on the
  `anthropic_cloud` environment, step 0's `CLAUDE_CODE_REMOTE` check comes
  first, and the pull request is its only way to report (see above).
- Tell it **not** to merge — merging is serialized and belongs to the session
  running the batch.
- Tell it not to start a heavy build while the machine's heavy-build slots are
  taken, to give it half the cores (`CARGO_BUILD_JOBS`) while another may
  overlap it, and not to run `make mutants` while siblings are compiling.
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
merged — that list is what the operator reads before updating the service.

## Learn from the batch: every lesson ends as a change

A batch is trial by fire: it finds the traps, gaps and bugs in the tooling and
in scorsese faster than anything else. A lesson that only reaches the chat is
lost, because the next session starts without it. So every lesson ends as
exactly one of:

- **fixed**: a pull request that removes the cause;
- **enforced**: a lint, gate or test that makes it impossible to repeat;
- **documented**: this skill, `ci-merge`, `issue-write`, `cloud-brief.md` or
  CLAUDE.md, for what the repo can't change (a platform behaviour, a procedure);
- **tracked**: an issue with the evidence (run ids, PR numbers, measurements) and
  a proposed fix, never just the symptom.

**Mid-batch, fix what blocks or bites twice.** When a lesson costs the running
batch time — a tooling bug that stalls the queue, a brief that makes every agent
re-solve the same trap — fix it now. It is infrastructure, so it outranks
whatever was next. A skill or doc edit is Markdown-only and merges without
queuing (except `docs/project-format.md`, which tests parse; `ci-merge` has
how). One that changes a **rule in `CLAUDE.md`** rather than recording a step
is a change to scorsese's conventions: open it and leave it for the user
(*Take the initiative*). On rusty's
2026-09-30 batch, five tooling and skill fixes landed within the hour their
lesson appeared (MatthewLacerda2/rusty#577).

**Otherwise, file it and keep going.** The strongest issues come from doing the
work. File an agent's "follow-up, not in this PR" notes and a PR's "decisions to
check" the moment you read them, not at the end.

**At the end, run a retro before reporting back:**

1. List every trap, gap, surprise, workaround and hand-back from the batch: your
   own notes, each PR's *Decisions* and *Not exercised* sections, the issues the
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
3. **Rebuild the local tools, and launch nothing.** `make release` builds
   `scorsese` and `scorsese-mcp` — the MCP server a local client is pointed at
   is `target/release/scorsese-mcp`, so until this runs every session drives the
   old code. Then `cargo build --release --manifest-path app/Cargo.toml` for the
   desktop app. One after the other, never beside a sibling's build. A client
   already connected keeps the old server process until it reconnects; say so
   in the report.

## Reporting back

The user is not reading the transcript of a batch. They take long — often hours,
usually overnight — and the transcript is, if anything, notes for Claude itself.

**When things go well, say what the result was.** When things did not go as one
would expect, say what the surprise was. That does not necessarily mean things
went badly: we write it down because the more we can predict, the better we
improve. Include the retro's lesson → change mapping.

Two things still interrupt, because they are the ones the user would want to
overrule and overruling is only possible while the batch is still running: **a
change to the user's own files** outside the repo, and **a decision reversed** —
where the issue said one thing and the branch did another.
