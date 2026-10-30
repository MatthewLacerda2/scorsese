# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt names
the issue; what is specific to the issue is the newest comment on it whose
first line is `Brief` (`issue-batch` has the template). When a lesson changes
how cloud coders work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Decisions and
hand-backs go in the description and an issue comment, never a push
notification. Use the GitHub MCP tools (load them with ToolSearch); don't
assume `gh` is installed.

**File only on a signal** (`CLAUDE.md`, *File an issue only on a signal*):
fold what you find into the branch when it is small or in your way; file it,
with the evidence and the `agent` label, only when it has bitten or will bite,
or costs more left than fixed; otherwise let it go, and keep going.

## Before you start

Read `CLAUDE.md`, your issue(s), your brief and the `ci-merge` skill. A later
`Brief` supersedes an earlier one. Check each "Blocked by" on GitHub yourself:
a blocker written only in prose is invisible to the orchestrator. If one is
open, stand down: no branch, a comment saying which should go first, and stop.

**Test first** (`docs/testing.md`): write the test and watch it fail, then the
code. For a bug, the branch's first commit is the test that reproduces it.
`make gates` cannot run CI's `regression test` job: a bug no `bun test` or
Rust test can reproduce (layout a Playwright flow proves) takes its
`No regression test on base: <why>` line in the description before ready.

## Ready means finished, and then the branch is not yours

- Never wait on CI: it runs once you mark the pull request ready, and the merge
  queue waits for it. Your proof is `make gates`, run here.
- A branch that changes a path `independent-review` names gets that review
  **before** you ready it, by that skill: you dispatch the reviewer, answer
  its findings, re-run the gates and post its comment.
- Mark it ready **only when the branch is truly finished**: it merges the
  moment CI is green. Never ready it just to get a CI run.
- **Once ready, never push to it again**, not even a rebase: the queue rebases
  it and refuses a head it did not watch. A red run goes to the orchestrator,
  not you; anything you find later goes in an issue comment, for a follow-up.

## The container

The `SessionStart` hook (`.claude/hooks/session-start.sh`) has installed what
`make gates` needs, started a native Postgres (`SCORSESE_TEST_DATABASE_URL`)
and exported `CARGO_INCREMENTAL=0` for the ~30 GB disk: don't override it,
except for `cargo mutants` (*The mutation signal*). If its `session-start:`
lines are missing, run it yourself
(`CLAUDE_PROJECT_DIR=$PWD .claude/hooks/session-start.sh`) and apply the
`export` lines it prints; a `FAIL` line says what to install.

- A call past ten minutes moves to the background and you are woken when it
  ends. Wait for it; don't end your turn with the branch half-proven.
- **Run each heavy gate once, into a log, and read the log** (`make test >
  target/test.log 2>&1; echo exit=$?`, then `grep`), never again just to see a
  summary you piped away.
- Stop any `dockerd` or containers your proof started before the gates, or
  they die of memory (exit 137). The app gate can too: `CARGO_BUILD_JOBS=2`.
- **GitHub's GraphQL and search APIs answer 403 here**, so `gh pr list` and
  `gh pr view --json` fail; REST and the MCP tools work. A script meant to run
  here uses REST.
- **The web image cannot be built here**: `bun install` fails behind the proxy
  (`SELF_SIGNED_CERT_IN_CHAIN`) inside it. In the container itself it works
  with `NODE_EXTRA_CA_CERTS=/root/.ccr/ca-bundle.crt` exported, which `make
  web-gates` needs too. Reproduce the image's layout and run `bun run build`
  there. If Playwright's Chromium cannot be fetched either, point
  `E2E_CHROMIUM` at the container's headless shell (`web/README.md`).
- **A headless browser** is the `playwright` MCP server (`.mcp.json`,
  `web/README.md`'s *A browser for agents*), loaded without asking. It runs
  `bun install` in `web/` and fetches Playwright's Chromium shell on its first
  start, or uses the repo's pinned shell when that host is out of reach. When
  its `browser_*` tools are missing, run `tools/playwright-mcp < /dev/null`
  to read why; Playwright's own runner (`bunx playwright test`) still works.
- A gate you cannot run (`make deploy` refused as a production deploy, `infra`
  refused the network) is named on the *Gates* line with the CI job that
  answers for it, never claimed green.
- Every container so far has been x86_64. If `uname -m` says otherwise, bless
  nothing.

## Blessing references

Only a reference **your branch adds or deliberately changes**, by name. Look at
every PNG you bless before committing, and list it in the pull request under a
`Re-blessed` heading with why: the queue hands back a changed reference it does
not name (`docs/golden-renders.md`, *Re-blessing*).

- **Render goldens**: run the others first and bless nothing if any fails; then
  `UPDATE_GOLDENS=1 cargo test -p scorsese-golden --test suite -- --exact
  goldens::<name>`, never the whole suite. An unintended mismatch is investigated, never
  re-blessed.
- **App panel snapshots** (`app/tests/panels`) are each cropped to the part
  they are named for, except `a_whole_edit` and `a_whole_edit_in_dark`, which
  move with any chrome change. Check each diff stays inside your change.

## Easy to miss

- **What you push is what CI builds.** Before readying, `git status --short
  --ignored` lists nothing you wrote: `make size` catches an ignored code file,
  but a fixture or a page is yours to check.
- **Numbers are taken last**, from `origin/main` + 1: a `schema_version` bump
  (with its migration step), `SYNTH_VERSION`, a SQL migration's `NNNN_`.
  Renumber if a sibling's lands first.
- `docs/project-format.md` is parsed by tests: editing it is a code change.
- **A change meant not to change behaviour is proven by comparison**: capture
  the output first (`tools/list`, a generated table, frames) and show it
  identical after.
- Regenerate generated text (`make mcp-table`), never hand-merge it, and run
  `cargo fmt --all` after any conflict.
- **A new dependency is a collision**: say so in the description, so the
  orchestrator can sequence it (`issue-batch`, *Lockfiles have one owner per
  wave*).

## The mutation signal

A branch that adds mechanism asks for a run before it is readied (`ci-merge`),
since nothing reports survivors after you; a run that fails to report is never
a reason to stay draft. Check the mutant count before running.

- **Inside `.cargo/mutants.toml`'s surface**: `python3
  .github/scripts/mutants/scope.py resolve '<your globs>' --out
  target/scope.json --diff target/scope.diff`, then `CARGO_INCREMENTAL=1
  cargo mutants --in-diff target/scope.diff --jobs 2`
  (`docs/mutation-testing.md`).
- **Outside it** (`crates/server`, the rest of `crates/render/src/run`, `app/`)
  it reports nothing. A server module tested in its own file: `cargo mutants
  --no-config --test-tool nextest -f <file> --test-package scorsese-server --
  --lib`, one `-f` per file, short files only (each mutant rebuilds the
  server). One tested from `crates/server/tests` pays the Postgres suite per
  mutant: skip it and name the tests that cover it.

Survivors in code you wrote: fix, exclude with a reason, or file (`ci-merge`).

## Protocol

- Branch as the brief names it, off the latest `origin/main`; open a **draft**
  pull request on the first commit, `(#N)` in the title. A Sonnet adds the
  **`minor`** label.
- **The body's first line is `Closes #N`** for every issue it closes, and `refs
  #N` for one it leaves open (GitHub matches the keyword anywhere, even
  negated). Describe the change however reads best, with two things the
  orchestrator relies on: a **Gates** line (what ran green, on which head, what
  did not and why), and what only a human can check, as a checklist under a
  heading starting **For a human**. When there is nothing, that is one
  sentence, not a checkbox. End with `🤖 Generated with [Claude
  Code](https://claude.com/claude-code)`.
- Commits end with your session's `Co-Authored-By:` trailer. Push often: a dead
  container loses what is not pushed.
- **After every push, `make pushed`**: it passes only when your last commit is
  the branch's head on GitHub, which is the pull request's. Never report a
  push, or ready the pull request, until it has passed.
- **You cannot delete a branch** from the container. Name a throwaway branch
  you pushed in the human checklist, for the orchestrator to delete.
- A decision the issue left open takes the default it, `CLAUDE.md` or Filmora 9
  points to, and the pull request says which and why.
- Rebase onto `origin/main` **once, last**, run `make gates`, then mark it
  ready. **Do NOT merge**, and never merge `main` into the branch: the
  queue cannot replay a merge commit, and a force-push may be refused. If yours
  is, push the rebase to `<branch>-rebased` and open a new pull request that
  supersedes the first. If the merges you rebased over touched none of your
  crates and the gates were green before, `cargo check --workspace
  --all-targets --locked`, `make size` and your own crates' tests are enough.
- ≈3 attempts at the same failure, or a decision that is genuinely the user's:
  leave it draft with the reason in the description and an issue comment, and
  stop.
