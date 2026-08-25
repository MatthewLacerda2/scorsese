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
correctness. The `mutants` job in `.github/workflows/ci.yml` is
`continue-on-error`, and nothing it finds can fail a build or block a merge.

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
and the plan and audio arithmetic of `crates/render`. The ffmpeg command builders
and `crates/golden` are excluded on purpose.

`.cargo/mutants.toml` is the authority on that list and gives the reason for
every inclusion and exclusion. Read it there rather than trusting this
paragraph — this one is a summary and the config is the thing that runs.

Per pull request the run is narrowed again with `--in-diff`, so the cost tracks
the size of the change rather than the size of the codebase.

## The scheduled sweep

`--in-diff` has a consequence worth naming: a line is audited **once**, on the
pull request that wrote it, and never again. A module whose tests were later
weakened, or whose assertions moved to another crate, has nothing looking at
it. So `.github/workflows/mutation-sweep.yml` sweeps the rest — no `--in-diff`,
the whole crate — every Monday, **one crate at a time, cycling**: `core`,
`compositor`, `render`, `zimmer`, which covers the surface every four weeks.

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
cargo mutants                                     # the whole scoped surface, 3676 mutants
cargo mutants -p scorsese-zimmer                  # one crate, as the sweep runs it
cargo mutants -F '^crates/core/src/keyframe\.rs'  # one file, while writing it
```

That 3676 moves with the source and with the tool version, and
`cargo mutants --list | wc -l` is how to re-read it: `--list` builds nothing
and runs nothing, so the count costs a second and is exact.

`-F` and not `-f` for that last one, and the difference is a trap worth
knowing: `--file` is *unioned* with the config's `examine_globs`, so
`-f one/file.rs` widens the run to everything rather than narrowing it to one
thing. `--re` filters the mutant names, which start with the path, so it does
narrow — but not perfectly. As of **cargo-mutants 27.1.0**, struct-field
deletions (`delete field … from struct …`) ignore the name filters entirely:
they are neither selected by `--re` nor removable by `--exclude-re`. Fourteen of
them live on the scoped surface, so every `-F` run carries all fourteen along
from wherever they are, and the report describes files you did not name. Read
past any `delete field` row from a file you did not ask about — or reach for
`-p scorsese-core`, which narrows to a whole crate with none of that, because
package and glob filters choose files before mutants exist. Nothing narrows to
exactly one file.

Results land in `mutants.out/` (gitignored). `mutants.out/missed.txt` is the
survivor list; `mutants.out/diff/` holds the actual edit that survived, which
is usually the fastest way to see what a survivor means.

`make mutants` finishes by rendering that as the Markdown CI posts — see
[Reading the report](#reading-the-report) for what it does and does not list.

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
  there were at least three. That is one statement about the module: *no test
  in the mutated crate asserts on this code.* The cause is usually structural.
  `test_workspace = false` runs only the mutated package's own tests, so a
  module whose assertions live in another crate has every mutant survive by
  construction, and no per-line reading of the list would have found that out.
  The response is one assertion next to the code, or one written reason it
  belongs elsewhere.
- **A file with more than eight survivors** — collapsed to a count whatever was
  caught in it. Past that length nobody triages the rows individually, and what
  the fifteenth says is what the first said.

Everything else is listed in full; the report does not paginate. A table with
rows in it is short because there is genuinely something to read.

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
`mutants` CI job's first step. A **floor** and not the count itself, so writing
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
