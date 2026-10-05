# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt carries
only step 0 (which runs before this file is read), a pointer here, and what is
specific to its issue (`issue-batch` has the template). When a lesson changes how
cloud coders must work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Never send the
operator a push notification: a hand-back or a decision goes in the PR
description and an issue comment. Use the GitHub MCP tools (`issue_read`,
`create_pull_request`, `update_pull_request`, `add_issue_comment`); load them
with ToolSearch. Don't assume `gh` is installed.

**A bug you find is yours to deal with.** Fix it in this branch when it's in
your way or small, or file an issue with the evidence (`issue-write`) and keep
going. Either way it never goes unrecorded. The operator expects coders to file
issues mid-batch.

## Before you start

Read `CLAUDE.md`, your issue(s) and `.claude/skills/ci-merge/SKILL.md`. Check
each "Blocked by" in the issue body on GitHub yourself — a blocker written only
in prose is invisible to the orchestrator (rusty#399). If one is open, stand
down: no branch, a comment saying which should go first, and stop.

## Ready means finished, and then the branch is not yours

- Run builds, tests and `make gates` in the **foreground**: a session that ends
  its turn to wait on a background build is never woken. You never wait on CI —
  it runs once you mark the PR ready, and the merge queue waits for it.
- Mark the PR ready **only when the branch is truly finished**; the orchestrator
  reads the diff and queues it, and it merges the moment CI is green. Never
  ready a PR to get a CI run: your proof is `make gates` here.
- **Once ready, never push to it again**, not even a rebase: the queue rebases
  it and refuses a head it did not watch (#745's coder raced the orchestrator
  this way). A red run goes to the orchestrator, not you. Anything found later
  goes in an issue comment, for a follow-up PR.

## The container

The `SessionStart` hook (`.claude/hooks/session-start.sh`, #722) has installed
everything `make gates` needs, started a native Postgres
(`SCORSESE_TEST_DATABASE_URL`) and exported `CARGO_PROFILE_DEV_DEBUG=0
CARGO_INCREMENTAL=0` (the disk is ~30 GB, #588: don't override them). Its
`session-start:` lines open the session; a `FAIL` line says what to install. If
they are missing, run it yourself (`CLAUDE_PROJECT_DIR=$PWD
.claude/hooks/session-start.sh`) and apply the `export` lines it prints.

- **A foreground call dies silently at ten minutes.** Cold, split it:
  `cargo nextest run --workspace --locked --no-run` (~5½ min), `make clippy`,
  then `make gates` (~4 min) — or its targets one per call (`make help`).
- **The app gate can be killed for memory** (exit 137): `CARGO_BUILD_JOBS=2`.
- **No docker daemon is needed**; start `dockerd` only to run a container.
- **Every container so far has been `x86_64`.** If `uname -m` says otherwise,
  `cargo install --locked` the tools, expect `grade_*`/`vhs` to fail, and bless
  nothing.
- A gate you cannot run is named in the PR with the CI job that answers for it,
  never claimed green.

## Blessing references

Only a reference **your branch adds or deliberately changes**, by name, and you
**look at** every PNG you bless before committing and explain it in the PR.

- **Render goldens**: run the others first and bless nothing if any fails;
  `UPDATE_GOLDENS=1 cargo test -p scorsese-golden --test goldens -- --exact
  <name>`, never on the whole suite. An unintended mismatch is investigated,
  never re-blessed (`docs/golden-renders.md`).
- **App panel snapshots** (`app/tests/panels`, #628): each is the whole window,
  so a change to a shared strip like the top bar moves every one with a project
  open (#726 re-blessed 12, #755). Check each diff stays inside your change.

## Easy to miss

- **Numbers are taken last**, from `origin/main` + 1: a `schema_version` bump
  (with its migration step), `SYNTH_VERSION`, and a SQL migration's `NNNN_`.
  Renumber if a sibling's lands first (#714 and #716 both took `0017`; the queue
  now hands that back, #729). A schema bump turns on the `web` and `app` gates.
- `docs/project-format.md` is parsed by tests: editing it is a code change.
- **A change meant not to change behaviour is proven by comparison**: capture
  the output first (`tools/list`, a generated table, frames) and show it
  identical after (#748, byte for byte).
- Regenerate generated text (`make mcp-table`), never hand-merge it, and run
  `cargo fmt --all` after any conflict (#686).
- **A new dependency is a collision.** If your issue needs one, say so in the PR
  description, so the orchestrator can sequence it behind whichever branch owns
  the lockfiles this wave (`issue-batch`, *Lockfiles have one owner per wave*).

## The mutation signal

Worth a run before readying when the branch adds mechanism — nothing reports
survivors after you (#651) — and never a reason to stay draft.

- **Inside `.cargo/mutants.toml`'s surface**: `python3
  .github/scripts/mutants-scope.py resolve '<your globs>' --out target/scope.json
  --diff target/scope.diff`, then `cargo mutants --in-diff target/scope.diff
  --jobs 2` (`docs/mutation-testing.md`).
- **Outside it** (much of `crates/server`, `crates/mcp`, `crates/render/src/run`,
  all of `app/`) that reports nothing: `cargo mutants --no-config -f <file>
  --test-package <crate> -- --lib`, one `-f` per file.
- **Server code needing Postgres per mutant** takes hours: skip it and name the
  tests that cover it. **Check the mutant count first** — one container saw its
  scope ignored (#740).

Survivors in code you wrote: fix, exclude with a reason, or file (`ci-merge`).

## Protocol

- Branch as the prompt names it, off the latest `origin/main`; open a **draft**
  PR on the first commit, `(#N)` in the title. Body: what changed / why / effect
  / decisions, `Closes #N` (only if it does: GitHub matches the keyword
  anywhere, even quoted or negated, so a PR leaving an issue open writes
  `refs #N`), a **Gates** line (what ran green, on which head, what did not and
  why), a checklist for anything only a human can check, ending with
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Commits end with your session's `Co-Authored-By:` trailer. Push often: a dead
  container loses what is not pushed.
- A decision the issue left open takes the default it, CLAUDE.md or Filmora 9
  points to, written under *Decisions*.
- Rebase onto `origin/main`, run `make gates`, then mark it **ready**.
  **Do NOT merge.**
- ≈3 attempts at the same failure, or a decision that is genuinely the user's:
  leave it draft with the reason in the description and an issue comment, and
  stop.
