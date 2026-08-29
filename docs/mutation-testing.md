# Mutation testing

Coverage answers *"was this line executed?"*. Mutation answers the harder and
more useful question: **"if this line were wrong, would anything notice?"** A
line can be executed by ten tests and asserted on by none.

That gap is not hypothetical here. Break-testing #8 found a volume-ramp test
that passed with volume ignored entirely: it asserted only that three measured
levels ascended, which a clip playing at a constant level satisfies by chance
about half the time. Replacing `Gain::at`'s body with a constant left that test
green — the definition of a surviving mutant.

#8 made the ramp assertions real, and the first full sweep then found the same
gap one layer down: the *arithmetic* under the ramp was still unpinned, and
`Mix::add` could mix by **subtraction** with the whole suite green. Closing that
(#60) took assertions of a different kind — the sample is those two sources
added, the multiplier is a quarter of the way from 0.0 to 1.0 — because a
measurement over a window is satisfied by more than one arithmetic. That is the
shape of the answer when a survivor is real: name the value, not the range.

The tool is [`cargo-mutants`](https://mutants.rs/). It edits one small thing in
the source — a returned value, a comparison, an arithmetic operator — rebuilds,
and runs the tests. A mutation the tests **catch** is a mutation something
asserts on. A mutation that **survives** is a change nobody objected to.

## It is a signal, never a gate

Per CLAUDE.md's gates-vs-signals rule, this audits quality; it does not prove
correctness. The `mutants` jobs in `.github/workflows/ci.yml` — the plan, the
shards, and the report they merge into — are all `continue-on-error`, and
nothing they find can fail a build or block a merge. What *can* turn one of
their checks red is the instrument being broken: a collapsed surface, a base
that cannot be resolved, or a report with no plan behind it to write from.

That is a deliberate design decision and not a soft start. Mutation produces
**equivalent mutants** — changes that alter the code without altering its
behaviour, which no test can possibly catch. A gate that regularly demands a
fix for something that is already fine teaches everyone to route around it, and
then it means nothing.

## What it does not do

It does not replace break-testing. Of the three real test gaps found in this
project so far, mutation would have caught one. The other two were *semantic*:
a music bed restarting its source at every cut instead of resuming, and
keyframes timed from the timeline instead of from the clip. In both, the line
ran, an assertion reached it, and no generated mutant would have survived —
because the bug was not "this code does nothing", it was "this code does a
plausible wrong thing among several plausible things". No tool mutates "resumed
from the wrong offset" into existence.

So the two are complements:

- **Mutation** — does anything assert this *at all*?
- **Break-testing** — does the assertion pin down the *right* behaviour?

## What gets mutated

The pure-logic surface only: `crates/core`, `crates/compositor`, `crates/zimmer`,
the plan and audio arithmetic of `crates/render`, and `crates/providers`'
`src/synth/` — the one subtree of that crate with no provider in it, which is
where a recipe is parsed, tuned, and turned into the address its bake is cached
under. The ffmpeg command builders and `crates/golden` are excluded on purpose,
and so is the rest of `crates/providers`.

`.cargo/mutants.toml` is the authority on that list and gives the reason for
every inclusion and exclusion. Read it there rather than trusting this
paragraph — this one is a summary and the config is the thing that runs.

Per pull request the run is narrowed again with `--in-diff`, so the cost tracks
the size of the change rather than the size of the codebase.

## Sharding, and the report a diff too large still gets

`--in-diff` makes the cost track the diff, and for a while that was the end of
it: a large enough diff simply ran past the job's `timeout-minutes: 30` and was
cancelled having said nothing at all. #383 is the case — 233 mutants, a whole
COLRv1 painter, **no report**. The signal was least available exactly where it
was most useful, and a cancelled job looks identical to a broken one, which
inverts what a red check here means.

So the job decides what it is about to do before it does any of it.
`cargo mutants --list --in-diff` builds nothing and answers in well under a
second, so the count of mutations in scope is free *up front*:

- **Under the budget** — one runner, exactly as before. Most pull requests.
- **Over it** — `cargo mutants --shard k/n` across up to four runners in
  parallel, sized from the count. 80 mutations per shard, from #394's measured
  ~10.5s per mutant on a runner against a 20-minute per-shard budget.
- **Over even that** — the shards run what they can and the report says what it
  did not reach. `.github/scripts/mutants-merge.py` puts the shards back
  together, and a shard that was stopped, or that never reported at all, leaves
  the merged run stamped as unfinished.

The 30 minutes was not raised, and raising it is not the fix: it is a judgement
about what a per-pull-request signal may cost, and a run needing an hour has
stopped being the thing the job is for. The budget is enforced *inside* the
step instead, so a shard that runs out of time ends by uploading what it
measured rather than by being killed with the report unwritten.

Nor is there a deliberate *sample*. Sharding is what makes one unnecessary, and
what is left when even sharding will not fit is not a designed subset — it is
whatever the clock allowed, reported as that and counted.

## The scheduled sweep

`--in-diff` has a consequence worth naming: a line is audited **once**, on the
pull request that wrote it, and never again. A module whose tests were later
weakened, or whose assertions moved to another crate, has nothing looking at
it. So `.github/workflows/mutation-sweep.yml` sweeps the rest — no `--in-diff`,
the whole crate — every Monday, **one crate at a time, cycling**: `core`,
`compositor`, `render`, `zimmer`, which covers those four every four weeks.

`providers`' `synth/` subtree is on the surface and **not** in that rotation,
which the workflow picks from a list of crate names. So those mutants are
audited by the pull request that writes them and never again — the very thing
the sweep exists to stop, in miniature. Whether that earns a fifth weekly slot
is #431; until it does, the sweep can be pointed at `scorsese-providers` by
hand from its `workflow_dispatch` input.

Rotation and not one big monthly run, for a reason that was measured rather
than assumed: the whole surface extrapolates to seven to ten hours on a
GitHub-hosted runner against a six-hour job limit, and a monthly cadence would
also miss the seven-day cache eviction and build cold every time. The workflow's
header carries the arithmetic and says plainly which half of it is a
measurement.

It reports into **one issue that rewrites itself** — [#341][sweep] — using the
same renderer the pull-request comment goes through. The report at the top is
replaced every run; the catch-rate table underneath only ever gains a row,
because one catch rate is a number and the question is whether it is moving.

A sweep that is cut short says so, in the report and in its history row. A
truncated sweep reporting as a complete one is the one outcome worse than no
sweep at all.

Same standing as the per-pull-request job: `continue-on-error`, nothing it
finds blocks anything, and a survivor it turns up is triaged exactly as below.

[sweep]: https://github.com/MatthewLacerda2/scorsese/issues/341

## Running it

```sh
cargo install cargo-mutants --locked

make mutants                                      # what CI runs: this branch's diff
cargo mutants                                     # the whole scoped surface, 3875 mutants
cargo mutants -p scorsese-zimmer                  # one crate, as the sweep runs it
cargo mutants -F '^crates/core/src/keyframe\.rs'  # one file, while writing it
```

That 3875 moves with the source and with the tool version, and
`cargo mutants --list | wc -l` is how to re-read it: `--list` builds nothing
and runs nothing, so the count costs a second and is exact.

`-F` and not `-f` for that last one, and the difference is a trap worth
knowing: `--file` is *unioned* with the config's `examine_globs`, so
`-f one/file.rs` widens the run to everything rather than narrowing it to one
thing. `--re` filters the mutant names, which start with the path, so it does
narrow — but not perfectly. As of **cargo-mutants 27.1.0**, struct-field
deletions (`delete field … from struct …`) ignore the name filters entirely:
they are neither selected by `--re` nor removable by `--exclude-re`.
Twenty-seven of them live on the scoped surface, so every `-F` run carries all
twenty-seven along from wherever they are, and the report describes files you
did not name. Read past any `delete field` row from a file you did not ask
about — or reach for
`-p scorsese-core`, which narrows to a whole crate with none of that, because
package and glob filters choose files before mutants exist. Nothing narrows to
exactly one file.

Results land in `mutants.out/` (gitignored). `mutants.out/missed.txt` is the
survivor list; `mutants.out/diff/` holds the actual edit that survived, which
is usually the fastest way to see what a survivor means.

`make mutants` finishes by rendering that as the Markdown CI posts — see
[Reading the report](#reading-the-report) for what it does and does not list.

**Where it builds:** not here. cargo-mutants copies the worktree into a scratch
directory and compiles every mutant in the copy, which is why a mutation run
does not disturb `target/` and why it needs somewhere to put several gigabytes.
Two settings keep that honest: `gitignore = true` in `.cargo/mutants.toml`, so
build output is left behind rather than copied — `app/target/` is the one that
matters, ~8.5 GB of a workspace no mutant lives in — and `TMPDIR`, which
`make mutants` points at `~/.cache/scorsese/mutants` because `/tmp` on the
development machine is a tmpfs and a copy that will not fit in RAM fails with a
message about disk (#392). A run that dies before reporting on any mutant says
which directory it was copying into.

**How much of the machine it takes:** two mutants at a time, not eight. Left to
itself cargo-mutants sets `--jobs` from the CPU count, and since each job is a
whole cargo build in its own copy of the worktree, that sizes the run by the
resource that is not scarce — what runs out is memory, and rustc's peak is
measured in gigabytes. On the development machine the default OOM-killed the
agent session twice in one day, and the kernel named the wrong thing when it
did: Claude Code runs with `oom_score_adj: 200`, so it is reaped as the
preferred victim while the compilers that ate the RAM carry on. The symptom is
a dead terminal, three layers from the cause (#398). All three callers pass
`--jobs 2` — `make mutants`, the CI job and the sweep — and the argument for
the number is written once, in `.cargo/mutants.toml` under *How wide a run fans
out*, because cargo-mutants has no config key to hold it. It is two *per
shard*, and stays two when the CI job fans out across several: each shard is
its own runner, so four of them do not share the cores and the memory the
number is sized against. A run that needs to
be gentler still than that: `make mutants MUTANTS_JOBS=1`.

The copy has no `target/` in it either — `copy_target` has been off by default
since 25.1.0, and before that 1.0.2 stopped copying it at all, because
cargo-mutants sets its own `RUSTFLAGS` and existing build products would not
match. So **nothing built in advance is reused.** Every mutant compiles from
scratch out of the crate downloads in `~/.cargo`, and those are the only thing
worth having ready. Both CI workflows used to run a `cargo build` before the
mutation step on the opposite assumption; #394 measured it on the runner —
mutation step and cargo-mutants' own baseline build unchanged, warmed or cold,
for about three minutes a run — and took it out.

**When to run it:** once the implementation is written and its tests pass, which
is where CLAUDE.md puts it. That is when a survivor is cheapest to answer — the
code is still in hand and the missing assertion is a two-minute edit — and it is
why the run is not left to CI, whose comment arrives on a pull request that has
already been declared finished.

`make gates` ends with a line about it: whether `make mutants` has been run on
this branch, and what it found if it has. It never runs it, because a signal
inside the target that gets run most would stop being run at all — so the line
is a report, and it says *not run* rather than going quiet over an answer it
does not have. It knows because `make mutants` leaves `target/mutants-signal`
behind, stamped with the branch it ran on and what the run found; a Rust file
changed after that stamp makes the line read *stale* instead.

## Reading the report

`python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json` renders
the run as the Markdown CI posts, and it is a **worklist**: rows are survivors
somebody can act on one at a time.

Two things never become rows, because they are one finding rather than many:

- **A file where nothing at all was caught** — every mutant of it survived, and
  there were at least three. That is one statement about the file: *nothing in
  the mutated package asserts on this code.* It is one piece of work, not a
  list — but **which** piece is not something the counts can tell you, and the
  report deliberately no longer guesses: *Triaging a file where nothing was
  caught*, below, has the three candidates and the command that decides.
- **A file with more than eight survivors** — collapsed to a count whatever was
  caught in it. Past that length nobody triages the rows individually, and what
  the fifteenth says is what the first said.

Everything else is listed in full; the report does not paginate. A table with
rows in it is short because there is genuinely something to read.

### A report that says it did not cover everything

Two banners, and they are separate claims:

- **"This run did not finish"** — cargo-mutants recorded a start and no end, so
  it was stopped rather than completed. For the sweep that is a crate outgrowing
  the six-hour job limit; for a pull request it is a shard reaching its budget.
- **"N of M mutations in this diff were not measured"** — how much of the diff
  nobody looked at, in mutations. Only the pull-request job prints it, because
  only it knows the number `--list --in-diff` gave before the run started.

A gap is **not** a survivor and it is not a catch. Nothing is known about those
mutations, and the absence of rows for them says nothing at all — which is the
point of printing the number rather than leaving it to be inferred. If the gap
covers code this branch wrote and the answer matters, `make mutants` locally has
no thirty-minute clock; the alternative is to say in the pull request which part
went unmeasured, so the next reader is not left to guess.

## Triaging a file where nothing was caught

**Check the cause before deciding what to do about it.** The report used to
name one — *usually structural, assertions living in another crate* — and it
was wrong both times it appeared: #442's six survivors were that branch's own
`#[serde(default)]` functions, tested from the same package and simply not
tested yet; #444's four were `Song::curve`, a public convenience method with
**zero callers anywhere in the workspace**. Two for two, two different real
causes, neither the one named. #449 took the guess out, because a wrong
diagnosis is worse than none: it points a reader at *exclude with a written
reason* before they have looked, and an exclusion carrying a plausible
argument is approximately permanent.

**The check is cheap.** `cargo mutants --list` names the functions and builds
nothing; in both cases above it answered in seconds. Run it, then grep the
workspace for the names it prints.

The three causes, in no particular order:

- **The code has no test.** Ordinary, and the fix is ordinary: one assertion
  next to the code. This is what #442 was.
- **The code has no callers.** Nothing in the workspace reaches it, which is
  why nothing asserts on it. **Deleting it is the fix**, and the mutants go
  with it — CLAUDE.md's *nothing in the codebase is temporary* and the
  `unreachable_pub` gate both say the same thing about `pub` nobody can reach.
  This is what #444 was, and a mutation report is a good place to notice it.
- **Its assertions live in another crate.** Real — `providers`' `record`
  carries exactly such a note — and `test_workspace = false` (see
  `.cargo/mutants.toml`) runs only the mutated package's own tests, so every
  mutant survives by construction. The response is one written reason, not a
  weakened test.

## Triaging a survivor

**A survivor is a test gap until it is shown to be equivalent.** In order:

1. **Read the mutation**, in `mutants.out/diff/`. It tells you what the code
   could have done differently with nothing complaining.
2. **Ask what behaviour that would change.** If you can describe an input where
   the mutated code produces a different — wrong — result, that is a real gap:
   write the test that asserts that result. Add it to the test file that
   already covers the function, not a new one named after the tool.
3. **If no input distinguishes them, the mutant is equivalent.** Common shapes:
   a value that gets clamped straight back into range, a default that another
   branch always overwrites, a `#[derive]`-adjacent helper with one caller.
   Record it as an `exclude_re` entry in `.cargo/mutants.toml` with a written
   reason.

### The shape that fools step 3: a flipped sign

**When a mutation flips a sign and nothing fails, look at what the assertions
measure before concluding the mutant is equivalent.** The pattern is one
sentence: *a measurement that discards sign cannot test an operation that
changes it.* Magnitudes, DFT bins, peaks, absolute values, rising-crossing
counts and render-to-render inequality all discard it, and every one of them
reads like a real assertion — which is exactly why step 3 above is tempting.

Four modules hit this in a single day, and the survivors were real gaps in all
four:

- `core/additive.rs` — fifteen tests read a one-bin DFT **magnitude**, which is
  invariant under a sign flip, so `+=` → `-=` walked through every one.
- `core/osc/mod.rs` — sixteen tests read peaks, rising-crossing counts, or
  equality between two renders. All three survive negation.
- `core/fm/four.rs` — two feedback tests each held one term at zero, so `+` and
  `-` were *literally identical* in both.
- `song/render.rs` — a humanise offset added to the tone: the test asserted the
  tone had **moved**, which a renderer subtracting the offset does just as far
  (#451).

The fix is never a new kind of test. It is asserting the direction the code
already claims: which of two renders is brighter, which of two onsets is
earlier, what the sample at a known instant actually is. Two coordinates chosen
so the correct code points opposite ways — a seed that draws up and one that
draws down, a note ahead of the beat and one behind it — turn "it changed" into
a claim a sign flip cannot satisfy.

One corollary worth stating, because it looks like a pass and is not: an
unsigned subtraction between two indices (`later - earlier` on `usize`) does
catch a flip, but it catches it as an **arithmetic overflow panic** rather than
as a failed assertion. Nobody wrote that catch, its message says nothing about
the mark or the offset it was checking, and a `saturating_sub` added later for
tidiness removes it silently. Assert the order, then measure the distance.

### Writing the exclusion: an entry is a regex

**`exclude_re` holds regular expressions, not the lines `cargo mutants --list`
prints.** The two look identical for most mutants, because most mutant names are
made of letters — and that is the trap. A mutation *of an operator* puts regex
metacharacters into the name, and `|` is the one that costs everything:
`replace || with && in unit` is an alternation with an empty branch, an empty
branch matches every string there is, and so that single entry excludes the
**whole surface**.

It is silent when it happens. The run reports a handful of mutants, finds no
survivors, and passes — which is also what a healthy branch touching no mutated
lines looks like. #363 and #365 were that bug, and it went unnoticed across ten
merged pull requests.

So, when writing an entry:

- Use a TOML **literal** string — single quotes, no escape processing — and
  backslash-escape every metacharacter in it: `'replace \|\| with && in unit'`.
- Check the surface afterwards with `cargo mutants --list | wc -l`. It builds
  nothing, and the answer is four figures. Six is what the config looks like
  when an entry has eaten everything.

That check is wired up rather than remembered: `.cargo/mutants.toml` records a
`surface-floor:` line, and `.github/scripts/mutation-surface.py` compares the
count against it — inside `make mutants` before it mutates anything, and as the
first thing the `mutants: plan` CI job does, before it has decided anything
about the diff at all. A **floor** and not the count itself, so writing
code never trips it and deleting the surface does. It is deliberately not part
of `make gates`: it proves the instrument works, not that the code is right.

If the surface genuinely shrinks — a crate deliberately dropped from
`examine_globs` — the floor moves in the same commit, with the reason. Lowering
it to quiet the check is the same move as re-blessing a golden reference to make
CI green.

One thing no entry can exclude, per `.cargo/mutants.toml`'s note on name
filters: a struct-field deletion (`delete field … from struct …`). An entry for
one is accepted, does nothing, and the mutant is still reported.

**Where the code draws, the value to name is a measurement.** The compositor's
survivors are nearly all one shape (#323, #350, #352): a test samples a pixel
inside a drawing, which says the drawing is roughly where it belongs and nothing
about it landing in *almost* the right place — a Bézier control point a few
pixels out, an origin computed by an addition. That is "name the value, not the
range" read visually, and the values are there to be named: the box a drawing's
ink occupies and the area it covers are both things the geometry can be asked for
on paper. `crates/compositor/tests/common/extent.rs` is that measurement, shared
by the shape, icon and blur tests rather than written three times.

What is never a correct response is **weakening or deleting an assertion** to
make the report quieter. A test changed to suit a tool has stopped testing the
code. This is the same rule as re-blessing a golden reference to make CI green
(see [golden-renders.md](golden-renders.md)) and it is broken the same way: by
treating a red signal as the problem instead of what it points at.

Fixing survivors is also not the job of the PR that surfaced them. A survivor
in code the PR did not write belongs in its own issue.
