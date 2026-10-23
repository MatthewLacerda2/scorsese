---
name: independent-review
description: Use before labelling `queue` on a scorsese pull request that changes money, stored data, security or web isolation, concurrency (jobs, cancel, the assistant's turn loop), the render pipeline, or the gates — `make mergeable` hands one back without it — and whenever the user asks for an open or merged pull request to be reviewed. Dispatches a fresh, read-only reviewer that never saw the coder's reasoning, then answers its findings on the pull request.
---

# Independent review

## Why it exists

Nobody reads this repository's code but agents (`CLAUDE.md`: the user does not
read it), so besides the gates a pull request gets the orchestrator's read of
the diff, and the orchestrator briefed the coder: it reads what it asked for.
A coder reviewing its own diff reviews what it meant to write. The reviewer is
a separate agent that reads the pull request the way an outsider would, and is
never told what to think of it. **Everything below exists to keep the coder's
bias out of the reviewer.**

The review itself is Claude Code's `code-review` skill, which moves with every
new model, so this skill says only when to run it, how to dispatch it without
bias, and what to do with what it finds. It never restates how to review.

## When

**The gate decides, by path** (`.github/scripts/reviewed.py`, `TRIGGERS`):
`make mergeable` and `make queue` hand back a pull request that changes one of
these without an `Independent review` comment at the level shown. Markdown
and test files never trigger; Dependabot is exempt. The record behind each:

| Area | Level | What it covers |
|---|---|---|
| money | `high` | rates, quotes, the spending ceiling, credits, Stripe, currency, every generate path (CLI, MCP, the web's tools, the desktop app's dialog), what a generation records |
| stored data | `high` | `project.json` migrations, the writer, Postgres migrations, the library, `assets gc`, import, backups |
| security | `high` | accounts, auth, the database's per-user scope, tokens, credentials, the capture sandbox, deploy and Terraform |
| concurrency | `medium` | the job queue, the assistant's turn loop (wake, compaction, quotes, stopping), MCP sessions, cancel, Veo polling |
| render | `medium` | the render run, pipes, frames, plan, page capture, audio, preview, time and timeline |
| the gates | `medium` | workflows, `.github/scripts`, the Makefile, `tools/lint`, the golden-render gate, hooks, test config |

The highest level any changed path asks for is the one the review runs at,
and a rename counts its old path too. One gap is deliberate: the root
`Cargo.toml` holds the lint policy but changes with every dependency, so a
change to `[workspace.lints]` is the orchestrator's call, like a refactor.
**Not by path, and the orchestrator's call: a refactor or a move** (no path
says *refactor*) takes `medium` even where nothing above triggers.

What does not get one: `minor` pull requests that touch none of these paths,
Markdown-only changes, a mechanical rebase, and the assistant's
prompt and wording, which are taste.

**A trigger can misfire** — a doc comment in `prices/`, a renamed import. Then
the description says so on a line of its own, `No independent review: <why>`,
and the gate reads the reason. It is an exception, written where every reader
sees it, never a way round a review the change needed.

And on demand: when the user asks for a pull request to be reviewed, open or
merged (*Catching up on a merged pull request*, below).

## Dispatch

- **A fresh agent**: the `Agent` tool with `subagent_type: "general-purpose"`.
  **Never `"fork"`**: a fork inherits the orchestrator's whole context, and
  with it the bias this exists to avoid. No `isolation: worktree` (it changes
  nothing) and no model override.
- **Read-only.** It may read the pull request, the issue and the repository,
  and run read-only commands (`gh pr view`, `gh pr diff`, `git show`, a test).
  It makes no edits, commits, pushes or comments.
- **The prompt carries only what an outsider would get**: the pull request
  number, its issue, the level, the steering for its areas and the template
  below. **Not** a summary of the change, the coder's rationale, what was
  already checked, which differences are "known and fine", or what you expect
  it to find. The description is fair game — it is what the author tells every
  reader — and the reviewer tests its claims rather than trusting them.
- **One reviewer per pull request**, all dispatched in the same message when
  several are ready. A reviewer only reads, so it counts against no limit
  (`issue-batch`, *Agents that only read and think*).
- **When:** once the pull request is ready and its gates are green, and before
  it is labelled `queue`. CI runs meanwhile; the review does not wait for it.

### Level

`code-review` takes an effort level from `low` (few, high-confidence findings)
to `max` (many, some uncertain). The level follows what a miss would cost, not
the size of the diff: **`high`** for money, stored data and security, where a
miss is a charge, a lost project or a leak; **`medium`** for everything else
the table names. **Never `xhigh` or `max`**: past `high` the findings turn
into suggestions, and suggestions are where Claude is weakest. A reviewer here
reports defects with a failure scenario; the coder decides what lands.

Always name the level in the prompt: called without one, `code-review` reuses
whatever level was typed last in the session.

### Prompt template

```
Review PR #<n> of MatthewLacerda2/scorsese. Issue #<i> is what it is meant to
deliver; read it with `gh issue view <i>`.

Run the `code-review` skill on PR #<n> — pass the PR number in its arguments —
at `<level>`, without `--comment` and without `--fix`. Called without the
number it reviews the checkout's own diff, which is not this PR: if what it
reviewed is not `gh pr diff <n>`, discard its findings and review that diff
yourself. The checkout is not on the PR's branch; read its files with
`gh pr diff <n>` and `git show <head sha>:<path>`. You are read-only: no
edits, commits, pushes or comments. Read-only commands are fine.

Look especially for: <every steering list below whose area the PR hits>.

Answer with one entry per finding: file and symbol, the defect in one
sentence, a concrete failure scenario (inputs or state → wrong result or
crash), and whether it must be fixed before merge. Verify each against the
code before reporting it. Report defects, not suggestions or style. An empty
list is a valid answer.
```

### Steering, by area

Paste every list whose area the pull request hits, and the last one always.

- **Money** — what is charged equals what was quoted, on every path that
  spends (CLI, MCP, the web assistant, the desktop app's generate); every take
  counts against the ceiling, a superseded, expired, in-flight or part-failed
  one included; a run that fails part-way says what it already spent; nothing
  paid for is bought twice for an unchanged brief, or overwritten (`CLAUDE.md`,
  *What was paid for is kept*); the web's money stays integer micro-dollars;
  a rate in the code matches `docs/prices.md`.
- **Stored data** — every path that writes a project leaves one that opens: a
  failed or cancelled run, a narration shorter than its clip, a cleanup; a
  format change carries its schema bump and a migration from every older
  version; nothing deletes what a clip references or what was paid for; no
  absolute path reaches `project.json`; a Postgres migration already merged is
  never edited.
- **Security** — who reaches the data after the change, on every path that
  serves it (HTTP, the web's MCP tools, the assistant, captures, caches); one
  user's files, voices or projects never reach another; a page capture stays
  offline (WebSocket and WebRTC included); a secret is never logged, returned
  or kept past its use; the hole's siblings closed too, not only the one
  reported; tests that prove the refusal, not only the allowance.
- **Concurrency** — a cancel stops everything it started (the compositor,
  ffmpeg, a capture, the mix); a stopped or superseded job is not reported as
  failed; a turn racing a wake, a compaction or a quote's answer; polling is
  bounded, never a busy-wait that costs money; one slow call cannot stall the
  others; temp names unique across threads; a test that could flake under load.
- **Render** — frame boundaries and rounding (a render above the timeline's
  fps, a segment's end, crop fractions against ffmpeg's); speed applied to a
  clip's source range; a pipe's stderr drained and a broken pipe said plainly;
  a failed render keeps the previous file; the preview and the final render
  agreeing.
- **The gates** — a broken change still fails the job (would this gate have
  gone red on the bug it is for?); an exclusion or a regex that matches
  everything; triggers and path filters still firing; `make gates` and CI
  running the same thing; skipped never read as green.
- **Every pull request** — the CLI, MCP and the web still accept, refuse and
  say the same thing; every description it touches (a tool's,
  a flag's, a doc's) is still true.

## Answering the findings

The coder answers: a local subagent is resumed with `SendMessage`, and this
session answers for what it wrote itself. A cloud coder has ended by the time
its pull request is ready (`issue-batch`, *The pull request is the report*),
so the orchestrator fixes its branch or briefs a coder of the model the branch
takes (`CLAUDE.md`, *Which model writes a branch*) with a new `Brief` comment.
Pushing these fixes is fine: the branch is not the queue's until it is
labelled. The reviewer can be wrong, so every finding is checked against the
code first, then:

1. **Must-fix** findings are fixed on the branch. A finding declined gets its
   reason. A finding that needs a decision no rule or earlier decision settles
   is written on the pull request and goes to the user at the next check-in;
   the pull request is not queued until it is answered.
2. The gates are re-run and the branch pushed.
3. **One comment on the pull request, its first line `## Independent review —
   <level>`**, lists every finding and what became of it: fixed (with the
   commit), declined (with why), or sent to the user. No findings is said too.
   It is what the gate reads, and the record that the review happened.
4. When the fixes rework the logic substantially, one more reviewer reads the
   new diff. Otherwise one round is the review.

Then it is labelled `queue`. The review does not replace the gates, CI or the
mutation signal (`ci-merge`); it runs beside them.

**Record it before the label.** A watch that hands a pull request back
remembers its head and does not take it again until something is pushed
(`ci-merge`, *The watch*); a comment is not a push.

## Catching up on a merged pull request

Same dispatch, on the merged pull request's number. Its fixes go in a **new
pull request** off `main` whose description names the reviewed one; the
`Independent review` comment goes on the reviewed pull request and links the
fix.

## Common mistakes

- Dispatching a `fork`: it is the coder again, with the coder's blind spots.
- Briefing the reviewer with the coder's summary "to save it time": it then
  reviews the summary, not the code.
- Letting the reviewer push its own fixes. It reports; the coder fixes.
- Leaving the level out of the prompt, or running everything at `high`.
- Letting `code-review` run without the pull request's number: it then
  reviews the checkout's local diff, some other change entirely.
- Applying every finding. The reviewer is a second opinion, not a work order;
  a declined finding with a reason is a good outcome.
