# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt carries
only step 0 (which runs before this file is read), a pointer here, and what is
specific to its issue (`issue-batch` has the template). When a lesson changes how
cloud coders must work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Use the GitHub
MCP tools (`issue_read`, `create_pull_request`, `update_pull_request`,
`add_issue_comment`); load them with ToolSearch. Don't assume `gh` is installed.

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
ones a fresh Linux container lacks. *(Not yet confirmed in a scorsese cloud
session: the first coder to hit a difference corrects this list in its PR.)*

- `make setup` once — the commit hook, and a check for `cargo-nextest`.
- `ffmpeg` and `cargo-nextest` for `make test`; `cargo-deny` for `make deny`.
- A Postgres for the server's tests: `tools/with-postgres` starts one in docker.
  Without docker, install Postgres and point `SCORSESE_TEST_DATABASE_URL` at it
  (the script's header has why an ambient `DATABASE_URL` is ignored).
- `make deploy` needs docker too. A gate you cannot run here is named in the
  PR with the CI job that answers for it, never claimed green (`ci-merge`,
  *Before marking a pull request ready* — including when that keeps it a
  draft).
- A branch touching `app/`: the libraries CI's `app` job installs
  (`.github/workflows/ci.yml`, the *Install the graphics, windowing and sound
  libraries* step). A branch touching `web/`: `bun`, the version pinned in
  `web/package.json`.
- If `cargo deny` cannot fetch the advisory database through the container's
  proxy, run it with `CARGO_NET_GIT_FETCH_WITH_CLI=true`.
- The container is Linux, so the pixel gate runs here (it is skipped on macOS).
  A golden-render mismatch is investigated, never re-blessed to go green
  (`docs/golden-renders.md`).
- **A new fixture is blessed here**, and only it. `UPDATE_GOLDENS=1` on the
  whole suite rewrites every fixture's references under this container's
  ffmpeg, so: run the existing fixtures first and bless nothing if any fails;
  bless the new one by name (`UPDATE_GOLDENS=1 cargo test -p scorsese-golden
  --test goldens -- --exact <name>`); make sure no other `expected/` changed;
  then **look at** the new frames (Read the PNGs) before committing. CI gates
  under Ubuntu 24.04's ffmpeg `6.1.1`; if `decoder.txt` names another, say so
  in the PR.

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
- Don't run `make mutants`; CI runs the mutation signal on the pull request.
- Rebase onto the latest `origin/main`, then `make gates` (foreground), before
  readying.

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
