---
name: ci-merge
description: Take a finished branch through to merged — gates, CI, the mutation signal, rebasing, and `make mergeable`. Use when a branch is ready, when a pull request has gone red, when a mutation report needs triage, or when merging anything into `main`.
---

# Getting a branch merged

The steps and the traps. `CLAUDE.md` states each rule in a line; this is how to
keep it.

## What a ready pull request claims

**A ready pull request claims it passes; a draft makes no such claim.** CI runs
on ready pull requests and on `main`, nowhere else, so a red run always means a
claim was broken. A draft is **how work survives** a session that ends badly;
the durable context is the **issue**, written to be read cold. Never rely on a
hand-back comment existing.

**Gates block on correctness; signals inform on quality.** Build, test,
`clippy -D warnings`, the golden renders and the lint are hard gates: green to
merge, no exceptions. Coverage, mutation testing and perf tracking are signals:
they run on a schedule or when asked, never as a job in a pull request's CI
run, and never block a merge. Coverage runs weekly on `main`; mutation sweeps
one crate a week and otherwise runs when asked (*The mutation signal* below).

**CI is a different computer.** GitHub-hosted `ubuntu-24.04`: cold, no GPU,
and very often a different ffmpeg build than the one that produced a frame
locally. "Works here" and "passes CI" are separate claims, which is why golden
renders compare frames with tolerance. Anything GPU-dependent can be built and
tried locally but **can never be a merge gate**.

## Before marking a pull request ready

`make gates` must be green. It runs every gate CI blocks on but `regression
test`, which builds the base and runs in CI alone (`docs/testing.md`); `make
help` lists them. `make setup`, once per clone, installs the pre-commit hook (formatting and
the lint) and the worktrees' shared compile cache.

The web, web-e2e and app gates are the only conditional ones: each reports
**skipped** (web-e2e says *not run*) when the branch touches nothing it
checks — `web/` for web, `web/` or `crates/` bar `crates/golden` for web-e2e,
`app/` for app. Skipped is the honest answer; never read it as green.

**Every test was written first and seen failing** (`docs/testing.md`): a test
that never failed has not been shown to test anything. A bug's first commit is
its reproducing test.

**A gate this machine cannot run is named, not claimed.** `test` needs docker
(or `SCORSESE_TEST_DATABASE_URL`) for the server's Postgres, `deploy` needs
docker, `app` needs the ALSA headers. Run the rest, and say in the pull request
which gate did not run here and which CI job answers for it (`fmt + clippy +
test`, `deploy config`, `desktop app`). That does not keep it a draft — unless
the branch changes what that gate checks (`crates/server`, `deploy/`, `app/`):
then it is proven on a machine that can run it, not left to CI.

**A change meant not to change behaviour is proven by comparison.** A refactor,
a move, a dependency swap: capture the output first (a `tools/list`, a generated
table, rendered frames), and show it identical after, or explain each difference
in the pull request. A green suite says the tests still pass; only the
comparison says nothing moved.

**A failure the machine caused is not the branch's.** `No space left on device`,
`Disk quota exceeded`, `Cannot allocate memory`, or rustc or the linker killed
(`signal: 9`) say nothing about the code: free disk or memory (a merged
worktree's `target/`, a sibling's build) and run it again — never "fix" code
that was never wrong. Builds and gates run from a worktree, **never from the
scratchpad**, which may be a tmpfs.

Nor is a failure `main` has too, on this platform. Before fixing a red gate the
branch did not touch, run it on `main`; if it fails there, it is a bug to cite
or file, not the branch's.

## The merge, one branch at a time

1. `git fetch origin && git rebase origin/main` in the branch's worktree.
2. Push with `--force-with-lease`.
3. Wait for CI **on the rebased head**.
4. `make mergeable PR=N`.
5. Merge. Then remove the worktree and delete the branch.

Merging is serialized because Rust is compiled: two branches can each be green
alone and break `main` together — a changed signature in one crate and a new
caller in another compile apart and not together. So **speculative CI and
parallel merging stay out**. The only exception is a pull request touching
**only** Markdown, whose CI run compiles nothing. A page the code reads is not
one of those: `.github/scripts/ci-paths.py` re-includes every page a test or an
`include_str!` reads.

Three constraints hold whatever automates the waiting: a conflict is never
resolved by a machine that cannot say *why* the code is shaped as it is; local
green is never CI green; and the mutation signal never becomes a precondition.
A run's verdict is about a **tree** and carries only to the same tree, so a
rebase is always re-run.

### `make queue` does steps 1–5, so nobody sits through step 3

    make queue WATCH=1            # a batch: whatever is cleared, as it is cleared
    make queue PRS="486 488 489"  # a fixed list, in the order given

Reach for it whenever more than one branch is finished at once. It merges one at
a time, re-fetching `main` between each; it asks `mergeable` about every branch
before merging it; and it stops on anything it cannot do safely:

- **Any conflict at all** — the branch is handed back with the conflicting
  paths named, and the queue moves on. It never picks a side.
- **A red run, an absent run, a moved head, a draft** — handed back, never
  merged, and the summary at the end says which and why.

It builds nothing locally and reads no mutation report. It does not remove
worktrees or delete branches (an agent may be standing in one), so step 5's
cleanup stays yours, and the summary lists what to clean. Doing it by hand is
still fine for a single branch.

### The watch: label a pull request `queue` and it merges

`make queue WATCH=1` takes no list. Every poll it asks GitHub for the open,
ready pull requests labelled **`queue`**, and takes the best one through the
same five steps:

- **`queue` is the go-ahead, and readiness is not.** A cloud coder readies its
  own pull request before anybody has read the diff. Whoever reviews it — the
  orchestrator, after reading the description and the diff, and seeing a risky
  change through `independent-review` — adds the label. Removing it pulls a
  branch back out before its turn.
- **Order is fewest conflicts first, then `CLAUDE.md`'s label priority, then
  age.** At each pick the watch counts how many of the others in line each pull
  request textually conflicts with and takes the lowest, so the broad pull
  request lands last and takes the one rebase. A tie goes to the label, read
  from the pull request and from the issues it closes. A pull request passed
  over three times goes first whatever its conflicts; the count is kept beside
  the lock, so it carries across resumed watches, and a new head starts it
  over. **Dependabot goes after
  all of them.** One that **conflicts with `main`** is handed back at once with
  the files named. A list run keeps the order it was given.
- **One queue at a time.** A second run on the same clone refuses with exit
  status 5, naming the holder's pid: follow the running watch's log, or stop
  that pid first. A `git fetch` of your own in the main checkout can collide
  with the watch's (`cannot lock ref`): harmless, run it again.
- **Dependabot labels its own pull requests `queue`**, all but GitHub Actions',
  which only ever proposes majors and waits for a person to read and label it.
  A red bot pull request is handed back like any other: read why, and fix it on
  `main` or close it. **Its branch is never pushed to by hand**: the queue
  comments `@dependabot rebase` when it is behind and waits for the new head; a
  change it needs is a pull request of its own off `main`. A bot pull request
  without the label means the *Sync labels* workflow never ran.
- **A hand-back stays out until its head changes.** Pushing a fix (or a rebase)
  is the whole of re-queueing it; one fixed in the description alone (an
  exception line) is re-queued with an empty commit.
- **It is bounded to fit the harness's two-hour background cap**: it takes new
  pull requests for `--for` minutes (default 70), the last one gets its own
  `--deadline` (40). Run it in the background and re-arm it when it exits; its
  last line, `watch: resume: <command>`, is the exact command to re-arm.
- **It reports the board, one `watch: ` line per event**: a pull request
  opening, turning ready, going back to draft or closing, and each one it
  finished with. `| grep --line-buffered '^watch: '` under a background
  `Monitor` is the whole of a batch's watching (`issue-batch`).
- **A watch runs the script it started with.** After a change to
  `.github/scripts/merge-*.py` merges, stop the watch (safe while it waits on
  CI), pull `main`, and start it again.
- **Exit status:** 4 is **unfinished** (a pull request still in line or a run
  still out at the deadline: run it again); 1 is a hand-back to read; 3 is
  GitHub unreachable after about three minutes of retries — run the same list
  again, nothing is merged or pushed twice. An unreachable GitHub is never a
  yes, for `make mergeable` either.

**Every ready pull request gets a run, Markdown-only ones included**, and every
run ends in **`CI ok`**, the one check the `main` ruleset requires. On a
Markdown-only change every other job skips, so the run is about a minute. A job
added to `ci.yml` gates `main` only once it is in `CI ok`'s `needs:`.

**A clean rebase can still push a broken head.** A merge ahead that changed a
signature this branch calls, added a field to a struct this branch builds by
literal, or pushed one of its files past the size cap rebases without a
conflict and fails CI ten minutes later. When the merges ahead touched the same
crates, rebase and `cargo check` the branch yourself first (`issue-batch` has
the loop). The queue does catch, before pushing, a migration number `main`
already holds and a `SCHEMA_VERSION` or `SYNTH_VERSION` bump `main` already
made.

**Once a pull request is in the queue, nobody pushes to it** except to fix its
own red run: the queue refuses to merge a head it did not watch.

## `make mergeable` is the gate, and its answer is final

It asks GitHub whether a run genuinely happened on the head commit. `gh pr
checks` is **not** a substitute: it blends runs, so a skipped run hides behind a
real one. Read its output before doing anything else: it tells apart the three
shapes seen in practice — a skipped run reading as green; no run because the
pull request was readied moments after a push (force one with an empty commit);
and no run because the branch conflicts with `main`, which reads exactly like a
broken workflow file (an invalid workflow still produces a `startup_failure`
run; no run at all means conflict, and the fix is the rebase).

Before reading any run it also refuses, and says what to write:

- a pull request closing an issue labelled `planning` or `human`, or whose
  closing keyword points at a pull request or a closed issue;
- a modified golden reference or app snapshot not named under a `Re-blessed`
  heading with a reason (`docs/golden-renders.md`, *Re-blessing*);
- a change to a risky path without an `Independent review` comment at the
  area's level (`independent-review`) — run it before the `queue` label, since
  a hand-back is only retaken after a push.

The `planning` label says the work is not ready. Claude never removes it on its
own; the user may tell it to remove the label and do the work. `make checks`
lists any open issue carrying both stage labels, for the user to say which
stays.

**Never hand-roll a "wait for CI" loop that treats zero checks as success.**
Absent and passing are different states: require checks to **exist** before
calling a run settled.

**The `CI` run holds gates only.** The one signal in it is the *Prices
(signal)* step, which can neither fail the run nor hold it. A job that audits
quality rather than proving correctness belongs in its own scheduled or
dispatched workflow.

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

**Two `schema_version` bumps are one number too few.** Two branches that both
bumped N → N+1 rebase the constant and every fixture's literal *cleanly* and
conflict only in `crates/core/src/migrate/mod.rs`'s `STEPS` and the step table
in `docs/project-format.md`. Keeping both there is wrong: the branch landing
second renumbers its step, its constant and its literals to the next version.
`SYNTH_VERSION` is the same: the answer is the next number, neither side.

**A lockfile conflict is never hand-merged.** Take `main`'s side of `Cargo.lock`
(or `app/Cargo.lock`) with `git checkout --ours` mid-rebase and let cargo put
this branch's edges back with `cargo metadata` (add `--manifest-path
app/Cargo.toml` for the app), which only adds what the manifests ask for. Then
`cargo check --all-targets --locked` for that workspace and `make deny` prove
the result before the push. A `Cargo.toml` conflict between two added
dependencies is the keep-both case.

After a rebase, re-check any claim the branch made **about the base it measured
against**. A byte-identity proof taken against an older `main` is stale, and
citing it is worse than not having run it.

## The mutation signal

It is a **signal, never a gate**: it cannot fail a build and no pull request
runs it. Asking is part of finishing a branch that adds mechanism: before
marking it ready,

    make mutants-remote SCOPE='crates/<crate>/src/<the files you wrote>/**'

or `SCOPE=diff` for everything the branch changed. It runs on GitHub's runners
and prints the report and each survivor's diff in the terminal. An exit of 1 is
**no report** — a red or cancelled run — and is never read as zero survivors;
re-dispatch once, then say in the pull request that the signal was not read.
Nothing reports survivors after the branch is readied, so this is the last
point one is cheap.

Read the report when it lists survivors in code **this branch wrote**. There are
three exits for a survivor and no fourth: **fix it**, **exclude it with a
written reason**, or **file it as its own issue**. `docs/mutation-testing.md`
has what the report deliberately does not list.

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
by reasoning about symmetry.

### Two patterns worth knowing before triaging

**A measurement that discards sign cannot test an operation that changes it.**
Magnitudes, DFT bins, peaks, absolute values, zero-crossing counts and
render-to-render equality all discard it, and every one reads like a real
assertion. A surviving `+` → `-` almost always means this.

**A boundary comparison surviving both `<=` and `>=` means neither end is
exercised.** Every instance so far has been a real gap.

A "0 of N caught, nothing at all" banner suggests the cause is structural
(assertions living in another crate). **Check before believing it**:
`cargo mutants --list` names the functions and builds nothing. The real causes
have been ordinary: code with no test, and code with no *callers* (which
deletion fixes, not a test).

`make mutants` is the same question asked of this machine: opt-in, and not to be
run while sibling agents are compiling. `make mutants-remote` is the default.

## `SYNTH_VERSION`

The rule and how to verify it are in `crates/zimmer/CLAUDE.md`. If you run
probes: **commit first**, then archive. An archive taken from a dirty tree
measures the wrong thing.
