# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt carries
only step 0 (which runs before this file is read), a pointer here, and what is
specific to its issue (`issue-batch` has the template). When a lesson changes how
cloud coders must work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Use the GitHub
MCP tools (`issue_read`, `create_pull_request`, `update_pull_request`,
`add_issue_comment`); load them with ToolSearch. Don't assume `gh` is installed.

**A bug you find is yours to deal with.** Fix it in this branch when it's in
your way or small, or file an issue with the evidence (`issue-write`) and keep
going. Either way it never goes unrecorded. The operator expects coders to file
issues mid-batch.

## Read first

`CLAUDE.md`, your issue(s), and `.claude/skills/ci-merge/SKILL.md`. The prompt may
name more.

## Push after every step: you share the account's usage limit

You run on the operator's account, so when it reaches its usage limit you stop
mid-turn, at the same moment as the orchestrator and every other coder, and
your container's unpushed work goes with you. On 2026-09-30 four coders stopped
together; one had just re-blessed nine golden references and lost them all.
Commit and push after each step that produced something: a passing test, a
re-bless, a rewritten description. A fresh session can finish a pushed branch;
it cannot recover an unpushed one.

## Never wait on background work without a wake-up

- Run builds, tests and `make gates` in the **foreground**, with long timeouts,
  split across calls when needed. A session that starts a build in the background
  and ends its turn to wait is never woken.
- If you must wait on CI, schedule your own check-in with `send_later` (the
  `Claude_Code_Remote` MCP tool) **before** ending the turn.

## Ready means finished

- Mark the PR ready **only when the branch is truly finished**: the orchestrator
  merges a ready PR the moment its CI is green.
- CI runs only on ready pull requests, so your proof on a draft is `make gates`
  run here, in the foreground. Never ready a PR just to get a CI run.
- **Once ready, the branch is the merge queue's.** Don't push to it again, and
  cancel any `send_later` check-in that would. The queue rebases, pushes and
  merges it, and it refuses to merge a head it did not watch, so a late push
  costs a full CI round. The one exception is fixing your own red CI. Anything
  else you find afterwards — a mutation survivor, a missing test — goes in a
  comment on the issue, for a follow-up PR off `main`.

## Getting `make gates` to run in a container

`make gates` checks for what it needs and says how to install it; these are the
ones a fresh Linux container lacks. *(Confirmed by scorsese's first cloud coder,
#619, on 2026-09-30: x86_64 Ubuntu 24.04, 4 cores, 15 GB, ~30 GB of disk. A
coder who finds a difference corrects this list in its PR.)*

- **Every foreground call is capped at ten minutes**, and a call cut off at the
  cap dies with no output. A cold first `make test` (a Postgres pull plus the
  whole suite) hit it. Split cold work — `cargo nextest run --workspace
  --no-run`, then the run — and send long output to a file. Warm, the whole
  `make gates` fits in one call (about four minutes).
- **The disk allowance is about 30 GB and a debug build fills it.** With debug
  info, the workspace's test build alone reached 29 GB and the linker died with
  `No space left on device` / `Bus error` (#588's coder). Build with
  `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0` exported for every cargo
  command, `make gates` included: the whole test build is then about 2 GB.
- **The app gate can be killed for memory** (exit 137, no output) when its
  clippy, build and tests link egui with every core. Run its steps with
  `CARGO_BUILD_JOBS=2` — `make help` and the `app-gates` target list them.
- `make setup` once — the commit hook, and a check for `cargo-nextest`.
- `ffmpeg` is absent: `apt-get install ffmpeg` gives `6.1.1-3ubuntu5`, CI's own.
  `cargo-nextest`, `cargo-deny` and `cargo-mutants` (27.1.0, the version CI
  pins) are absent too: `cargo install --locked` all three, about six minutes.
  Faster for two of them, the release tarballs on GitHub download through the
  proxy (`get.nexte.st` answers 403): nextest's
  `cargo-nextest-<v>-x86_64-unknown-linux-gnu.tar.gz` and cargo-deny's
  `cargo-deny-<v>-x86_64-unknown-linux-musl.tar.gz`. cargo-deny must be
  **0.19 or newer** — 0.18 cannot parse the advisory database's CVSS 4.0
  entries.
- A Postgres for the server's tests: `tools/with-postgres` starts one in docker.
  The docker CLI and compose plugin are installed but **the daemon is not
  running** — start it yourself (`dockerd > /tmp/dockerd.log 2>&1 &`, as root)
  before `make test` or `make deploy`. Docker Hub rate-limits the container's
  address (a second pull minutes after the first got `429`), so pull
  `postgres:17-alpine` once and rely on the cache. Without docker, install
  Postgres and point `SCORSESE_TEST_DATABASE_URL` at it (the script's header has
  why an ambient `DATABASE_URL` is ignored).
- `make deploy` needs docker too. A gate you cannot run here is named in the
  PR with the CI job that answers for it, never claimed green (`ci-merge`,
  *Before marking a pull request ready* — including when that keeps it a
  draft).
- A branch touching `app/`: the libraries CI's `app` job installs
  (`.github/workflows/ci.yml`, the *Install the graphics, windowing and sound
  libraries* step). A branch touching `web/`: `bun`, the version pinned in
  `web/package.json`.
- If `cargo deny` cannot fetch the advisory database through the container's
  proxy, run it with `CARGO_NET_GIT_FETCH_WITH_CLI=true`. When that fails too
  (it did for #588), `git clone --depth 1` the database into
  `~/.cargo/advisory-dbs/advisory-db-3157b0e258782691` and run the gate's
  command with `--disable-fetch`, in the root and in `app/`.
- **When Docker Hub answers 429**, the image has a native fallback: Postgres 16
  is installed. `pg_ctlcluster 16 main start`, set the `postgres` user's
  password, and export `SCORSESE_TEST_DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`.
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
- Green gates and a rebased branch → mark the PR **ready for review**.
- **Do NOT merge.**
- ≈3 attempts at the same failure, or a decision that is genuinely the user's:
  leave the PR as draft with the reason in its description, comment on the issue,
  and stop.
