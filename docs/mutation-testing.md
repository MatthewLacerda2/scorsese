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

The pure-logic surface only: `crates/core`, `crates/compositor`, and the plan
and audio arithmetic of `crates/render`. The ffmpeg command builders and
`crates/golden` are excluded on purpose.

`.cargo/mutants.toml` is the authority on that list and gives the reason for
every inclusion and exclusion. Read it there rather than trusting this
paragraph — this one is a summary and the config is the thing that runs.

Per pull request the run is narrowed again with `--in-diff`, so the cost tracks
the size of the change rather than the size of the codebase.

## Running it

```sh
cargo install cargo-mutants --locked

make mutants                                      # what CI runs: this branch's diff
cargo mutants                                     # the whole scoped surface, ~16 min
cargo mutants -F '^crates/core/src/keyframe\.rs'  # one file, while writing it
```

`-F` and not `-f` for that last one, and the difference is a trap worth
knowing: `--file` is *unioned* with the config's `examine_globs`, so
`-f one/file.rs` widens the run to everything rather than narrowing it to one
thing. `--re` filters the mutant names, which start with the path, so it
narrows as expected.

Results land in `mutants.out/` (gitignored). `mutants.out/missed.txt` is the
survivor list; `mutants.out/diff/` holds the actual edit that survived, which
is usually the fastest way to see what a survivor means.

`make mutants` finishes by rendering that as the Markdown CI posts — see
[Reading the report](#reading-the-report) for what it does and does not list.

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

What is never a correct response is **weakening or deleting an assertion** to
make the report quieter. A test changed to suit a tool has stopped testing the
code. This is the same rule as re-blessing a golden reference to make CI green
(see [golden-renders.md](golden-renders.md)) and it is broken the same way: by
treating a red signal as the problem instead of what it points at.

Fixing survivors is also not the job of the PR that surfaced them. A survivor
in code the PR did not write belongs in its own issue.
