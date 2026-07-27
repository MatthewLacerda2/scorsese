# Mutation testing

Coverage answers *"was this line executed?"*. Mutation answers the harder and
more useful question: **"if this line were wrong, would anything notice?"** A
line can be executed by ten tests and asserted on by none.

That gap is not hypothetical here. Break-testing #8 found a volume-ramp test
that passed with volume ignored entirely: it asserted only that three measured
levels ascended, which a clip playing at a constant level satisfies by chance
about half the time. Replace `Gain::at`'s body with a constant and that test
still passes — the definition of a surviving mutant.

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

cargo mutants                                     # the whole scoped surface
cargo mutants --in-diff <(git diff origin/main)   # what CI runs on a PR
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

`python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json` renders
it as the Markdown CI posts.

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
