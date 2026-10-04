# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt carries
only step 0 (which runs before this file is read), a pointer here, and what is
specific to its issue (`issue-batch` has the template). When a lesson changes how
cloud coders must work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Never send the
operator a push notification: a hand-back or a decision goes in the PR
description and an issue comment, where the orchestrator reads it and decides.
The operator checks in from the terminal, and a phone alert is noise. Use the
GitHub MCP tools (`issue_read`, `create_pull_request`, `update_pull_request`,
`add_issue_comment`); load them with ToolSearch. Don't assume `gh` is installed.

**A bug you find is yours to deal with.** Fix it in this branch when it's in
your way or small, or file an issue with the evidence (`issue-write`) and keep
going. Either way it never goes unrecorded — and the same goes for a design gap
or a missing feature you find (CLAUDE.md, *File what you notice*). Every issue
you file carries the `agent` label. The operator expects coders to file issues
mid-batch.

## Check the issue's blockers yourself

Before writing code, read the issue body for "Blocked by" and check each
blocker's state on GitHub. The orchestrator sees only GitHub's recorded
relationships, and a blocker written only in prose slips past it (on rusty,
MatthewLacerda2/rusty#399). If a blocker is still open, stand down: no branch,
a comment on the issue saying which issue should go first, and stop.

## Read first

`CLAUDE.md`, your issue(s), and `.claude/skills/ci-merge/SKILL.md`. The prompt may
name more.

## Never wait on background work without a wake-up

- Run builds, tests and `make gates` in the **foreground**, with long timeouts,
  split across calls when needed. A session that starts a build in the background
  and ends its turn to wait is never woken.
- If you must wait on CI, schedule your own check-in with `send_later` (the
  `Claude_Code_Remote` MCP tool) **before** ending the turn.

## Ready means finished

- Mark the PR ready **only when the branch is truly finished**: the orchestrator
  merges a ready PR the moment its CI is green, once it has read the diff
  and labelled it `queue`.
- CI runs only on ready pull requests, so your proof on a draft is `make gates`
  run here, in the foreground. Never ready a PR just to get a CI run.
- **Once ready, the branch is the merge queue's.** Don't push to it again, and
  cancel any `send_later` check-in that would. The queue rebases, pushes and
  merges it, and it refuses to merge a head it did not watch, so a late push
  costs a full CI round. The one exception is fixing your own red CI. Anything
  else you find afterwards — a mutation survivor, a missing test — goes in a
  comment on the issue, for a follow-up PR off `main`.

## Getting `make gates` to run in a container

**The `SessionStart` hook has already set the container up**
(`.claude/hooks/session-start.sh`, #722): the toolchain, the commit hook,
ffmpeg, the prebuilt `cargo-nextest`, `cargo-deny` and `cargo-mutants` at the
versions CI pins, `bun`, the `app/` libraries, a native Postgres 16 with
`SCORSESE_TEST_DATABASE_URL` pointing at it, and `CARGO_PROFILE_DEV_DEBUG=0
CARGO_INCREMENTAL=0` exported (a debug build with debug info filled the ~30 GB
disk allowance, #588 — don't override them). Its `session-start:` lines open the
session; a `FAIL` line says what to install by hand. If they are missing, run it
yourself: `CLAUDE_PROJECT_DIR=$PWD .claude/hooks/session-start.sh`, then the
`export` lines it prints. What it leaves to you:

- **Every foreground call is capped at ten minutes**, and a call cut off at the
  cap dies with no output. Cold, split the work and send output to a file:
  `cargo nextest run --workspace --locked --no-run` (about 5½ minutes on 4
  cores), `make clippy` (under a minute), then `make gates` (about 4 minutes).
- **The app gate can be killed for memory** (exit 137, no output) when its
  clippy, build and tests link egui with every core. Run its steps with
  `CARGO_BUILD_JOBS=2` — `make help` and the `app-gates` target list them.
- **No docker daemon is needed**: `make deploy` only parses the compose file,
  and the tests use the native Postgres. Start `dockerd` (as root,
  `dockerd > /tmp/dockerd.log 2>&1 &`) only for work that runs a container, and
  expect Docker Hub to answer `429` on a second pull.
- The prebuilt tools are `x86_64` only. On `aarch64` the hook reports them
  failed: `cargo install --locked` them, about six minutes.
- If `cargo deny` cannot fetch the advisory database through the proxy, run it
  with `CARGO_NET_GIT_FETCH_WITH_CLI=true`. When that fails too (it did for
  #588), `git clone --depth 1` the database into
  `~/.cargo/advisory-dbs/advisory-db-3157b0e258782691` and run the gate's
  command with `--disable-fetch`, in the root and in `app/`.
- A gate you cannot run here is named in the PR with the CI job that answers
  for it, never claimed green (`ci-merge`, *Before marking a pull request
  ready* — including when that keeps it a draft).
- The container is Linux, so the pixel gate runs here (it is skipped on macOS);
  on `aarch64` rather than CI's `x86_64`, `grade_*` and `vhs` fail regardless.
  A golden-render mismatch is investigated, never re-blessed to go green
  (`docs/golden-renders.md`).
- **A fixture the branch adds or deliberately changes is blessed here**, and
  nothing else — if `uname -m` says `x86_64`, CI's architecture. On `aarch64`,
  don't bless: arm64 float paths drift from x86_64 on new code even where every
  existing fixture passes. Say so in the PR and leave the fixture unblessed for
  the orchestrator. `UPDATE_GOLDENS=1` on the whole suite rewrites every
  fixture's references, so: run the others first and bless nothing if any
  fails; bless each by name (`UPDATE_GOLDENS=1 cargo test -p scorsese-golden
  --test goldens -- --exact <name>`); make sure no other `expected/` changed;
  then **look at** the frames (Read the PNGs) before committing. A changed
  reference is explained in the PR (`docs/golden-renders.md`, *Re-blessing*).
  CI gates under Ubuntu 24.04's ffmpeg `6.1.1`; if `decoder.txt` names another,
  say so in the PR.

## The repo's rules that trip cloud coders

- Source files ≤ 300 **lines of code**, test files ≤ 150; group by subfolder,
  never by filename prefix.
- No real provider call in any test, ever. Never run `make live-check` — it
  spends money, and a cloud session has no keys.
- A change to what a recipe renders to bumps `SYNTH_VERSION` in the same commit;
  a `schema_version` bump ships with its migration (CLAUDE.md has both). Bump
  **last**, to `origin/main`'s version + 1, and renumber if a sibling's bump
  lands first (`ci-merge`, *Rebasing*). The bump edits test fixtures under
  `web/` and `app/`, so their gates run: install what they need.
- `docs/project-format.md` is parsed by tests: an edit to it is a code change.
- `missing_docs` is a gate: every new `pub` item gets a doc comment.
- **A mutation run before readying** is worth it when the branch adds
  mechanism, and it is the last chance: pull-request CI runs no mutation and
  nothing reports survivors after you (#651). It is a signal — never a reason
  to stay draft. `make mutants-remote` needs an authenticated `gh`, which a
  cloud container does not have, so run it here: nothing else compiles in this
  container. Scope it to your files with a whole-file diff — `-f` widens the
  run and `--re` lets every *delete field* mutant on the surface through
  (`docs/mutation-testing.md`, *Running it*):
  `python3 .github/scripts/mutants-scope.py resolve '<your globs>' --out
  target/scope.json --diff target/scope.diff`, then `cargo mutants --in-diff
  target/scope.diff --jobs 2`. `make mutants` does the same for the whole
  branch diff.
- Rebase onto the latest `origin/main`, then `make gates` (foreground), before
  readying.
- **Rebasing a PR that is already ready: run the full `make gates` before you push.** A push to a ready PR is a CI run, and a red one is a broken claim. On 2026-10-03 #686's rebase pushed a hand-merged list that `cargo fmt --check` refused, and CI found it instead of the session. After resolving any conflict, run `cargo fmt --all`. Regenerate generated tables (`make mcp-table`) instead of hand-merging them.

## Protocol

- Branch as the prompt names it, off the latest `origin/main`.
- Open a **draft** PR on the first commit. Title carries `(#N)`. Body: what
  changed / why / effect / decisions, `Closes #N` (one line per issue), and it ends
  with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Commit messages end with the `Co-Authored-By:` trailer your session's
  attribution instructions give.
- Push often: a dead container takes its uncommitted work with it.
- A decision the issue left open: take the default the issue, CLAUDE.md or
  Filmora 9 (for taste) points to, and write it under *Decisions*. A check only a
  human can do goes in the description as a checklist; it never keeps the PR a
  draft.
- The description has a **Gates** line: which gates ran green here, on which
  head, and which did not run and why (`web`/`app` skipped, a gate this
  container could not run and the CI job that answers for it). A ready pull
  request claims it passes; this is where the claim is written down for the
  reviewer (two of 2026-10-03's PRs left it out).
- Green gates and a rebased branch → mark the PR **ready for review**.
- **Do NOT merge.**
- ≈3 attempts at the same failure, or a decision that is genuinely the user's:
  leave the PR as draft with the reason in its description, comment on the issue,
  and stop.
