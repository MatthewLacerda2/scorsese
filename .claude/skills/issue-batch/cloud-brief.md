# Cloud brief: the standing rules

Every cloud coder the batch launches reads this file. Its routine prompt carries
only step 0 (which runs before this file is read), a pointer here, and the issue
number; what is specific to the issue is a `Brief` comment on it (#820,
`issue-batch` has the template). When a lesson changes how
cloud coders must work, change it **here**, once, not in the next prompt.

**Nobody can talk to you; the pull request is your only report.** Never send the
operator a push notification: a hand-back or a decision goes in the PR
description and an issue comment. Use the GitHub MCP tools (`issue_read`,
`create_pull_request`, `update_pull_request`, `add_issue_comment`); load them
with ToolSearch. Don't assume `gh` is installed.

**Nothing you find goes unrecorded** (CLAUDE.md, *File what you notice*). A
bug, a missing feature, a design gap or a quality-of-life fix is folded into
this branch when it's in your way or small, or filed as an issue with the
evidence (`issue-write`) while you keep going. A change that alters how the user
sees or understands their data or project, changes stored data or needs a
migration is never folded in: file it with `planning`. Every issue you file
carries the `agent` label. The operator expects coders to file issues mid-batch.

## Before you start

Read `CLAUDE.md`, your issue(s), your brief and
`.claude/skills/ci-merge/SKILL.md`. **Your brief is the newest comment on your
issue whose first line is `Brief`**: your branch name, what to build on, the
siblings whose files to stay out of, and decisions already made. A later `Brief`
supersedes an earlier one, and it may name more to read. Check
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
  On 2026-10-06 that limit was the coders' single largest time sink: #781 lost
  two ten-minute `make gates` calls to it before splitting them.
- **Run each heavy gate once, into a log, and read the log.** `make test >
  target/test.log 2>&1; echo exit=$?`, then `grep` it. Never run a gate again
  only to see a summary you piped away: #851's coder re-ran `make test` for its
  summary line and spent another ten minutes.
- **Stop what your proof started before the gates.** #852's `make gates` died
  with exit 137, and the session's worker restarted, while its proof's
  `dockerd` and containers were still up. `docker rm -f` them and stop
  `dockerd` first.
- **`make deploy` can be refused by the session's permission check** as a
  "production deploy" (#778's coder, 2026-10-06; #852's ran it fine). It only
  validates `compose.yaml`. If it is refused, name it on the *Gates* line:
  CI's `deploy config` job answers for it.
- **The app gate can be killed for memory** (exit 137): `CARGO_BUILD_JOBS=2`.
- **No docker daemon is needed**; start `dockerd` only to run a container.
- **GitHub's GraphQL and search APIs answer 403 here** (2026-10-06, #821's
  coder): `gh pr list`, `gh pr view --json` and `search/issues` fail, while
  REST (`gh api repos/{owner}/{repo}/…`) and the GitHub MCP tools work. A
  script meant to run here too uses REST.
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
- **App panel snapshots** (`app/tests/panels`, #628): each is cropped to the
  part it is named for (#755), except `a_whole_edit` and `a_whole_edit_in_light`,
  which are whole on purpose — a change to the chrome moves those two and the
  pictures of that part. Check each diff stays inside your change.

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
- **Outside it** — `crates/server`, the pipe-holding rest of
  `crates/render/src/run`, `app/` (#760 has why) — that reports nothing. A
  server module whose tests sit in its own file needs no Postgres: `cargo
  mutants --no-config -f <file> --test-package scorsese-server -- --lib`, one
  `-f` per file — but each mutant rebuilds the server (5-10 min here, #760), so
  count first and run only a short file. One tested from `crates/server/tests`
  also pays the Postgres suite (~3 min) per mutant: skip it and name the tests
  that cover it. **Check the mutant count
  first** — one container saw its scope ignored (#740).

Survivors in code you wrote: fix, exclude with a reason, or file (`ci-merge`).

## Protocol

- Branch as the prompt names it, off the latest `origin/main`; open a **draft**
  PR on the first commit, `(#N)` in the title. **The body's first line is
  `Closes #N`** for every issue it closes (two of the 2026-10-06 batch's nine
  PRs left it out, and an issue its PR does not name stays open after the
  merge). Only an issue it really closes: GitHub matches the keyword anywhere,
  even quoted or negated, so a PR leaving an issue open writes `refs #N`. Then:
  what changed / why / effect / decisions, a **Gates** line (what ran green, on which head, what did not and
  why), a checklist for anything only a human can check, ending with
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- **You cannot delete a branch** from the container (the proxy refuses it, through git and the API alike). A throwaway branch you pushed (a measurement, a probe) is named in the PR's human checklist for the orchestrator to delete (#772's `772-measure`).
- Commits end with your session's `Co-Authored-By:` trailer. Push often: a dead
  container loses what is not pushed.
- A decision the issue left open takes the default it, CLAUDE.md or Filmora 9
  points to, written under *Decisions*.
- Rebase onto `origin/main`, run `make gates`, then mark it **ready**.
  **Do NOT merge.** `main` moves every fifteen minutes or so during a batch,
  so rebase **once, last**, not after every merge you notice. If the merges you
  rebased over touched none of your crates, `cargo check --workspace
  --all-targets --locked`, `make size` and your own crates' tests are enough
  after the rebase. The queue rebases again anyway, and CI checks that tree. On
  2026-10-06 four coders re-ran the full gates after a rebase over unrelated
  merges, about ten minutes each, and none of those runs found anything.
- A human checklist with nothing in it is one sentence, not a checkbox:
  `make checks` lists every unticked box (#835 and #860 wrote "- [ ] Nothing
  needs a human").
- ≈3 attempts at the same failure, or a decision that is genuinely the user's:
  leave it draft with the reason in the description and an issue comment, and
  stop.
