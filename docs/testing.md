# Testing

How a test behaves is already held elsewhere — offline, providers faked behind
their traits, frames compared with tolerance, a size cap, a timeout. This page
is the other half (#1508): **what deserves a test, of which kind, and when it
is written.** A test written after the code, by the coder who just wrote it,
tends to fit that code; it reaches the line and pins nothing, which is the gap
mutation testing keeps finding (`docs/mutation-testing.md`). So the test comes
first.

## Which kind of test, by layer

| Layer | Test |
|---|---|
| Pure logic (`core`, `compositor`, `zimmer`, the plan and audio arithmetic of `render`) | Unit tests that assert the exact value, never a range (`docs/mutation-testing.md`). |
| Rendering | Golden frames compared with tolerance (`docs/golden-renders.md`). |
| Vendors (`providers`) | The fake behind the trait: the request built and the reply parsed, never a network. |
| Tool surface (`mcp`) | A contract test per tool: what it accepts, what it refuses and the words it refuses with, its description. |
| HTTP (`server`) | Through the real router over loopback, a real Postgres per test; the refusal proved, not only the allowance. |
| Web front-end | `bun test` for a component; a Playwright flow for anything crossing the server (`web/README.md`). |
| Desktop app | Panel snapshots (`app/README.md`). |
| Gates and CI scripts (`.github/scripts`, `tools/lint`) | A case the check must flag and one it must pass, as for any check (`CLAUDE.md`). |

**Where the logic lives decides the test.** An endpoint with logic of its own
is the smell — the server has none by design — so that logic moves a layer
down and is tested there, as a unit, and the endpoint's test proves only that
it is wired: what it allows and what it refuses.

## Test first

Write the test, **watch it fail**, then write the code that makes it pass. A
test that never failed has not been shown to test anything.

For a bug, the reproducing test is the branch's **first commit**: it fails on
`main`, and the fix after it turns it green. Write it in a test file — a
`tests` directory, a `*.test.ts(x)`, a `test_*.py`, or a golden fixture — not
in a `#[cfg(test)]` module beside the code, so CI can lay it over the base
(next section).

No machine can tell which came first for a feature, so for a feature this is
the working rule, and the skills say it where a coder starts: `cloud-brief.md`,
`issue-batch`'s briefing of a local subagent, and `ci-merge`'s *Before marking
a pull request ready*.

## A bug fix proves itself

A pull request that closes an issue labelled `bug` must add or change a test
that **fails on the base**. CI's `regression test` job checks it, and `CI ok`
waits for it; `.github/scripts/regression.py` is the whole decision, and its
module doc the detail.

- **Which pull requests.** Those whose description closes (`Closes #N`, the
  keywords `make mergeable` reads) an issue labelled `bug` when the run starts.
  A Markdown-only change runs no test job, this one included.
- **What it runs.** The test files the pull request changed, as the size gate
  classifies tests (`tools/lint/src/classify.rs`), plus the golden fixtures
  under `crates/golden/fixtures/`, are copied over a checkout of the base, and
  each target they name runs there: a Cargo integration-test target, one Python
  test file, one `bun test` file. A helper module or a fixture under `tests/`
  is laid over too, but names no target alone.
- **What passes.** One target failing on the base. A target that does not
  build or import there does not count: a test calling the API the fix added
  would otherwise pass any fix.
- **The exception.** A bug no test can see (taste, a real window, real
  speakers), or a test that cannot exist without the fix's new API, is written
  on a line of the description, with the reason:

      No regression test on base: the stutter shows only in a real window.

A target is the unit, not a single test: an unchanged test in the same target
failing on the base also passes the check. `main` is green, so one failing
there is one the laid-over files broke.
