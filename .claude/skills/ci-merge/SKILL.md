---
name: ci-merge
description: Take a finished branch through to merged — gates, CI, the mutation signal, rebasing, and `make mergeable`. Use when a branch is ready, when a pull request has gone red, when a mutation report needs triage, or when merging anything into `main`.
---

# Getting a branch merged

The steps, and the traps. `CLAUDE.md` carries why these exist; this is how.

## Before marking a pull request ready

`make gates` must be green. It runs every gate CI blocks on — format, size, the
signal renderers, clippy, docs, tests, supply chain, the deploy's compose file,
the web front-end and the desktop app's workspace. `make help` lists them.

The web and app gates are the only conditional ones, and each reports
**skipped** when the branch touches nothing under `web/` or `app/`. Skipped is
the honest answer; never read it as green.

**A gate this machine cannot run is named, not claimed.** `test` needs docker
(or `SCORSESE_TEST_DATABASE_URL`) for the server's Postgres, `deploy` needs
docker, `app` needs the ALSA headers — and some machines may not run a service
at all. Run the rest, and say in the pull request which gate did not run here
and which CI job answers for it (`fmt + clippy + test`, `deploy config`, `desktop
app`). That does not keep it a draft — unless the branch changes what that
gate checks (`crates/server`, `deploy/`, `app/`): then it is proven on a
machine that can run it, not left to CI.

Deliberately **not** before every push. Checkpoint commits stay cheap — the
pre-commit hook is formatting and the size gate only, well under a second.

**A failure the machine caused is not the branch's.** `No space left on device`,
`Disk quota exceeded`, `Cannot allocate memory`, or rustc or the linker killed
(`signal: 9`) say nothing about the code: free disk or memory (a merged
worktree's `target/`, a sibling's build) and run it again — never "fix" code
that was never wrong. On rusty a cold build filled a tmpfs scratchpad and three
healthy branches were reported broken (MatthewLacerda2/rusty#580). So builds and
gates run from a worktree, **never from the scratchpad**, which may be a tmpfs.

Nor is a failure `main` has too, on this platform. Before fixing a red gate the
branch did not touch, run it on `main`; if it fails there, it is a bug to cite
or file, not the branch's (on 2026-09-30 every agent on the Mac re-argued five
app panel snapshots that fail on macOS regardless — #597).

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

### `make queue` does steps 1–5, so nobody sits through step 3

    make queue WATCH=1            # a batch: whatever is cleared, as it is cleared
    make queue PRS="486 488 489"  # a fixed list, in the order given

Reach for it whenever more than one branch is finished at once — that is the
case where step 3 costs ten minutes per branch and an agent holds a worktree
open through every one of them. It merges one at a time, re-fetching `main`
between each; it asks `mergeable` about every branch before merging it; and it
stops on anything it cannot do safely:

- **Any conflict at all** — the branch is handed back with the conflicting
  paths named, and the queue moves on. It never picks a side. `SYNTH_VERSION`
  is the standing reason: the right answer there is *the next number*, which is
  neither side.
- **A red run, an absent run, a moved head, a draft** — handed back, never
  merged, and the summary at the end says which and why.

It builds nothing locally (`make gates` is still yours to run before marking a
pull request ready) and reads no mutation report. It also does not remove
worktrees or delete branches — an agent may be standing in one — so step 5's
cleanup stays yours, and the summary lists what to clean.

Doing it by hand is still fine for a single branch. The script's own docstring
has the reasoning, including why it does not try to *skip* CI runs instead.

### The watch: label a pull request `queue` and it merges

`make queue WATCH=1` takes no list. Every poll it asks GitHub for the open,
ready pull requests labelled **`queue`**, and takes the best one through the
same five steps (#690):

- **`queue` is the go-ahead, and readiness is not.** A cloud coder readies its
  own pull request before anybody has read the diff, so *ready* only says the
  coder thinks it is finished. Whoever reviews it — the orchestrator, after
  reading the description and the diff — adds the label. An approving review
  was the alternative and is not available: these pull requests are opened
  under the owner's account, and GitHub refuses an author's own approval.
  Removing the label pulls a branch back out before its turn.
- **Order is fewest conflicts first, then `CLAUDE.md`'s label priority, then
  age** (#738). At each pick the watch counts how many of the others in line
  each pull request textually conflicts with (`git merge-tree --write-tree` per
  pair, no checkout) and takes the lowest: reordering cannot make a conflict go
  away, but it decides how many hand-backs a set costs, so the broad pull
  request lands last and takes the one rebase. A tie goes to the label —
  infrastructure, architecture, bug, foundation, feature, with documentation
  alongside the first, read from the pull request and from the issues it
  closes, since it is usually the issue that carries it. When the count
  overrides the label, the watch says so in one line (`#A goes before #B, …`).
  A pair git cannot answer about counts as no conflict: the order is an
  optimisation and never stops the queue. **Dependabot goes after all of
  them**, whatever its labels or conflicts say. A list run keeps the order it
  was given.
- **Dependabot labels its own pull requests `queue`** (`.github/dependabot.yml`,
  #721) — all but GitHub Actions', which only ever proposes majors and waits
  for a person to read and label it. That is the decision: they are monthly, grouped, semver-compatible
  bumps (Cargo lockfile-only, majors ignored) with nothing in the diff to read
  beyond what CI checks, so nobody labels them by hand, and the order above
  keeps them from taking a turn from the batch. A bot pull request that is red
  is handed back like any other: read why, and fix it on `main` or close it.
  **Its branch is never force-pushed** — Dependabot stops maintaining a branch
  somebody else pushed to — so the queue comments `@dependabot rebase` when it
  is behind and waits for Dependabot's new head, within the same `--deadline`.
  Don't push to a Dependabot branch by hand either; a change it needs is a
  pull request of its own off `main`, after which Dependabot rebases.
- **A hand-back stays out until its head changes.** The label stays on, and
  the watch skips the branch while it sits on the head it was handed back on.
  Pushing a fix (or a rebase) is the whole of re-queueing it.
- **It is bounded to fit the harness's two-hour background cap**: it takes new
  pull requests for `--for` minutes (default 70), the last one gets its own
  `--deadline` (40), and the two together may not pass 115 — the script
  refuses a pair that would. Run it in the background and re-arm it when it
  exits; a fresh watch reads the same labels, so nothing is lost between two.
- **A watch runs the script it started with.** Merging a change to
  `.github/scripts/merge-*.py` does not reach a watch already running: stop it
  (it is safe to while it only waits on CI — the restart sees the head it
  pushed and goes on waiting), pull `main`, and start it again. On 2026-10-04
  a watch started before #731 would have force-pushed Dependabot's branches
  had its bot pull requests not been unlabelled first.
- **Unfinished is not red.** A pull request still in line when the watch ends,
  or whose run is still out at its deadline, is reported **unfinished** and the
  exit status is **4**: run the queue again. 1 still means read a hand-back,
  and 3 still means GitHub stopped answering. A list run reports a run still
  out at its deadline the same way.

The label lives in `.github/labels.json`, with `dependencies` beside it for
Dependabot; a new clone of the repository gets both from the *Sync labels*
workflow (manual dispatch). Dependabot silently drops a label the repository
does not have, so a bot pull request without `queue` means the sync never ran.

**A queue that loses the network says so, and says it is not a hand-back.**
Every question the queue asks GitHub retries a failure in transit — a TLS
handshake or other timeout, a reset connection, a DNS failure, a 5xx, a 429 —
for about three minutes before giving up. A 404 or 422 is an answer and is not
asked twice, and the merge call is never retried (a retried merge that already
happened is a second one). When the retries run out the queue stops, still
prints its summary with that pull request as **unreachable** and the rest as
**not reached**, and exits with status **3** rather than 1; `make mergeable`
does the same, and an unreachable GitHub is never a yes. Status 3 means run the
same list again — a PR it already merged is skipped as merged, and a head it
already pushed is not pushed again. Status 1 is a real hand-back: read why
(#615; before it, one TLS timeout killed the queue three times on 2026-09-30).

**A Markdown-only pull request gets no run**, so `make queue` hands it back as
absent and `make mergeable` cannot say yes. It is the one merge done by hand:
check `gh pr diff N --name-only` is all `.md` and not `docs/project-format.md`,
that GitHub reports it mergeable, then `gh pr merge N --squash`. Don't label
one `queue`: the watch would spend five minutes of everybody's turn learning
there is no run, and hand it back.

**Because it builds nothing, a clean rebase can still push a broken head.** A
merge ahead that changed a signature this branch calls, or pushed one of its
files past the size cap, rebases without a conflict and fails CI ten minutes
later. When the merges ahead touched the same crates, rebase and `cargo check`
the branch yourself first (`issue-batch` has the loop); the queue then finds
nothing to rebase, pushes nothing, and only waits and merges. Two numbering collisions it does catch before pushing (#729): a migration number `main` already holds, and a `SCHEMA_VERSION` bump `main` already made — handed back unpushed with the number to move to.

**Once a pull request is in the queue, nobody pushes to it** except to fix its
own red run: the queue refuses to merge a head it did not watch, so a late push
is a hand-back and a full CI round. Under the watch, that fix is also what
re-queues it — a new head is taken again on its own.

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

**The `CI` run holds gates only.** `make mergeable` waits until every `CI` run
on the head has completed, and since #651 nothing in that run is a coverage or
mutation job, so the wait is the gates' and nothing else's. A job added to
`ci.yml` that audits quality rather than proving correctness would put a signal
back on every merge's clock; it belongs in its own scheduled or dispatched
workflow instead.

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
bumped 35 → 36 rebase the constant and every fixture's literal *cleanly* — the
same edit on both sides — and conflict only in `migrate.rs`'s `STEPS` and the
step table in `docs/project-format.md`. Keeping both there is wrong: the branch
landing second renumbers its step, its constant and its literals to the next
version (`every_version_since_the_oldest_has_exactly_one_step` fails if it
does not). It is `SYNTH_VERSION`'s rule again — the answer is neither side.

**A lockfile conflict is never hand-merged.** Two branches that each add a
dependency, or a branch over a Dependabot bump, conflict in `Cargo.lock` (or
`app/Cargo.lock`): take `main`'s side (`git checkout --ours` mid-rebase) and
let cargo put this branch's edges back with `cargo metadata` (add
`--manifest-path app/Cargo.toml` for the app), which only adds what the
manifests ask for. Then `cargo check --all-targets --locked` for that
workspace and `make deny` prove the result before the push. A
`Cargo.toml` conflict between two added dependencies is the keep-both case.
(2026-10-04: #747 over Dependabot's #737, #748 over #747.)

After a rebase, re-check any claim the branch made **about the base it measured
against**. A byte-identity proof taken against an older `main` is stale, and
citing it is worse than not having run it.

## The mutation signal

It is a **signal, never a gate**. It cannot fail a build and **it does not hold a
merge** — and no pull request runs it (#651). It runs when asked, so asking is
part of finishing a branch that adds mechanism: before marking it ready,

    make mutants-remote SCOPE='crates/<crate>/src/<the files you wrote>/**'

or `SCOPE=diff` for everything the branch changed. It runs on GitHub's runners
and prints the report and each survivor's diff in the terminal. An exit of 1 is
**no report** — a red or cancelled run — and is never read as zero survivors;
re-dispatch once, then say in the pull request that the signal was not read.
Nothing reports survivors after the branch is readied, and the queue merges
without looking, so this is the last point one is cheap.

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

`make mutants` is the same question asked of this machine: opt-in, never part of
passing, and not to be run while sibling agents are compiling — it fans out and
the machine cannot carry it. That is why `make mutants-remote` is the default.

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
