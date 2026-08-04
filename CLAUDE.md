# CLAUDE.md

**scorsese** is a cross-platform video editor built for agentic workflows — a
human or a Claude agent assembles a video from a JSON project file and renders
it headlessly, and a GUI exists for humans to scrub, tweak, and review.

Use plain language with the user and explain things at a high level; go into
the nitty-gritty of Rust, ffmpeg, or the compositor only when it's needed to
address something or for the user to understand what is going on. You can
still speak in a technical, detailed way when writing issues and PRs — those
are documentation left for a future Claude to understand what is being
planned/done. Tell the user when something is bottlenecking you. Don't recite
the rules of this file unless one is blocking you or explains a conclusion.

## North star

Scorsese must allow the user to fully realize what he envisioned for the video,
in the fastest way possible. Claude must be able to interface with Scorsese to
fully realize the video, given the user's idea for it. Scorsese's GUI is merely
an interface for the user to visualize the editing planning and process, Claude
must be able to carry the whole editing process with just the user's prompts and
necessary file resources provided by the user. Any add/edit to the codebase MUST
add to this vision.

### What scorsese is, and is not

Scorsese is a tool for **creating and editing videos** — cuts, titles, music,
narration, pacing. It is **not** a compositing suite: no node graphs, no
tracking, no rotoscoping, no colour pipelines, no VFX. Those are a different
craft and a different program.

The word to hold onto is **approachable**. If a capability would only make
sense to someone who already edits professionally, it is probably out of
scope; if it is something a person with an idea and some footage would reach
for, it is probably in.

**Filmora 9 is the reference for taste**, not a specification — when a
question is "what should this feel like?" rather than "what should this do?",
that is where to look. Worth copying in spirit: its built-in animations and
fonts, its audio controls (volume as plain linear ramps between points), and
speed changes on video and audio clips. Worth ignoring: anything that exists
because a professional demanded it.

This is a scope rule, so it cuts both ways. It is a reason to *refuse* an
elaborate feature, and equally a reason to *build* an obvious one well.

## Start here

- **docs/project-format.md** — the `project.json` schema: assets, tracks,
  clips, keyframes, paths, and what validation checks.
- **docs/recipes.md** — the synthesis recipe format: what to write in
  `recipes/*.json` to get an effect or a score out of `scorsese synth`. Free,
  offline, deterministic — read it before reaching for a sound file.
- **docs/golden-renders.md** — the pixel gate: what a fixture is, how frames are
  compared, and when re-blessing a reference is legitimate. Read it before
  changing anything a render's output depends on.
- **docs/output-formats.md** — the containers and codecs a render delivers in,
  which combinations are refused, and why that list is deliberately short.
- **docs/credentials.md** — where a provider key comes from, in one order for
  every way scorsese is run, and the spending ceiling that lives beside it.
- **docs/mcp.md** — the tools `scorsese-mcp` exposes, how a client is pointed
  at it, and the rule that every tool and every argument describes itself.
- Crate boundaries live in each crate's `lib.rs` module doc — read them before
  adding a dependency between crates.

## Architecture — decided, do not redesign

These decisions are settled. Changing one is `architecture`-label work, not a
side effect of a feature PR.

- **A project is a directory** (`*.scor/`): `project.json` + `assets/`
  (imported media, copied on import) + `generated/` (provider and synthesis
  output, content-addressed by the hash of its brief) + `recipes/` (authored
  synthesis documents — not rebuildable, deleting one loses work) + `cache/`
  (rebuildable, gitignored). All paths inside `project.json` are relative to
  the project root. **No absolute paths, ever** — a project must survive
  `scp -r` between machines.
- **Assets are entities, clips are references.** `project.json` has an assets
  table (id, kind, path, sha256 hash, probed metadata); tracks hold clips that
  reference assets **by id — never by path**. Asset kinds: `video`, `image`,
  `audio`, `text`, `generated_video` (Veo prompt), `generated_audio`
  (ElevenLabs TTS prompt), `synth_audio` (a synthesis recipe).
- **Generated clips and the sketch lifecycle.** A generated asset carries a
  **brief** and a state: `sketch → queued → generated → stale` (stale = brief
  edited after generation). Sketch/stale clips render as slug cards so a full
  preview cut costs $0. "GO" realises only sketch/stale assets, and output is
  cached by the hash of its brief — never redone for an unchanged one.
- **Two kinds of brief, and the difference is load-bearing.** A `prompt` is a
  sentence handed to a provider: it costs money, needs a network, and cannot
  be reproduced from the project alone. A `recipe` is a document in
  `recipes/`: synthesis reads it locally, for free, deterministically. An
  asset carries exactly the brief its kind takes and never the other. In code
  that is `AssetKind::is_prompted` vs `is_synthesized`, both under
  `is_generated` — do not collapse them back together.
- **Compositing is ours; ffmpeg only decodes and encodes** (Path B). ffmpeg
  decodes sources to raw frames → our compositor produces each output frame
  (transforms, alpha, text) → raw frames are piped to ffmpeg stdin for encode.
  Render settings (aspect, resolution, fps, bitrate) are user-chosen per
  render.
- **Keyframes are generic.** A keyframe track is
  `(property_path, [(t, value, easing)])` over any numeric property. Position,
  scale, opacity come first; the mechanism must not know which property it
  animates.
- **Generality rule: core defines property types, never property values.** We
  make text color choosable; we never write "make text red".
- **Audio is first-class.** Audio tracks with keyframable volume; auto-ducking
  of music under narration is a planned feature, not an afterthought.
- **`core` and `cli` never touch a display.** No window, no GPU surface, no
  GUI toolkit — that invariant is stated in their `lib.rs` docs and enforced
  in review. The GUI is a client of the same library logic the CLI and MCP
  server use.
- **The GUI is `egui`, in Rust, and deliberately thin.** One language and one
  toolchain, and a compositor frame reaches the screen as a texture rather
  than crossing a process boundary — which is what makes scrubbing feel
  immediate. It gets the operations a person reaches for *with a mouse, often*:
  scrub, select, nudge, trim, change a plain value. Anything with structure to
  it is a sentence to an assistant over MCP, not a menu. A GUI rich enough to
  do the editing would be a second, weaker way to do everything.
- **Ship it simple, then iterate from use.** The GUI is the one part of this
  project whose requirements cannot be reasoned out in advance — so the bar for
  a first version is *the user can start editing with it*, not *it is right*.
  Perfecting an interaction nobody has tried yet is the failure mode to avoid.
- **MCP is a protocol, not a Claude feature.** `crates/mcp` speaks MCP to
  whatever client is on the other end; Gemini, GPT and anything else that
  speaks it get the same tools, the same way an HTTP API does not care whether
  a browser, a phone or curl is calling. Claude is who we develop and test
  against, not a dependency. Nothing in the server may assume otherwise.

### Crate map

`crates/core` (model, serde format, validation) ← `crates/compositor` (frame
rendering, CPU tiny-skia first) ← `crates/render` (ffmpeg orchestration) ;
`crates/zimmer` (synthesis: recipe documents → samples; **no I/O at all**,
and no dependency on `core`) ← `crates/providers` (Veo + ElevenLabs +
synthesis, brief-hash cache) ; `crates/cli` (the headless `scorsese` binary) ;
`crates/mcp` (MCP server, thin wrapper over the same logic) ; `crates/golden`
(test infrastructure: the golden-render gate, which nothing ships and nothing
depends on) ; `app/` (the egui desktop app — its own cargo workspace, so a
graphics dependency tree never slows `cargo test --workspace`). Each `lib.rs` doc
states what its crate must never depend on — those boundaries are enforced in
review.

**`zimmer` is ours, and its name is the joke told twice.** The project is named
for Martin Scorsese; the crate that writes the music is named for Hans Zimmer,
surname only both times. It is not a vendor, not a third-party dependency and
not an acronym. The name deliberately says nothing about what the crate does —
the first line of its own `lib.rs` doc does that instead (*sound from a
document*), and `missing_docs` is a merge gate, so that line cannot quietly go
missing. **The rename stops at the crate boundary**: `scorsese synth`, the
`synth_*` MCP tools and `"kind": "synth_audio"` all stay, because an
agent-facing surface has to describe itself and the asset kind is the format
contract.

## How we work

- **The gates (push back before you build).** An idea becomes an issue only
  when all three hold; if any fails, **push back instead of complying**:
  1. **Understanding.** Claude actually understands the idea — the user has a
     clear intent and Claude can restate it. If unsure, restate it back and
     confirm before proceeding; don't guess.
  2. **Value.** The issue adds real value to the project. No busywork, no
     features for their own sake.
  3. **Craft.** It follows Rust good practices and the gold standards of
     video-editor architecture (the decided architecture above included). If
     it doesn't, say so and propose the right shape.
- **Flow:** idea → issue → branch → PR → CI green → merge. New work starts as
  an issue, not a surprise diff, and the PR references the issue it closes.
  **Issue-less PRs are allowed only** for documentation updates or bug fixes;
  everything else starts as an issue. Either way the PR description still has
  to clear the three gates.
- **Pull requests — open early, draft until ready.** The moment a branch has
  its first commit, open a PR for it — as a **draft**. Draft while in progress
  or blocked (say why in the description); **ready for review** once done and
  nothing further is needed from the user. The description says **what
  changed and the effect** — not process; how you got there appears only when
  it's needed to understand the diff.
- **A ready pull request claims it passes; a draft makes no such claim.** That
  is what the two states mean here, and **CI runs on ready pull requests and on
  `main`, nowhere else.** A draft is not decoration on unfinished work — it is
  how work survives, and all three cases it covers are ones where pushing is
  the point: a genuine question surfaced mid-issue and needs the user, CI went
  red and the ≈3-attempts rule fired, or the run was stopped or died mid-work.
  None of them asserts the work is finished, so there is nothing for CI to
  check; a red run therefore always means a claim was broken, which is worth a
  notification every time.
  - **Run `make gates` before marking a pull request ready** — deliberately
    *not* before every push. GitHub should not be the first thing that compiles
    the code, and checkpoint commits have to stay cheap, which is the same
    reasoning that keeps the pre-commit hook to formatting and the size gate.
  - **A red ready pull request stays ready** and is fixed in the next commit. It
    does not go back to draft. The next commit usually resolves it, so the real
    cost is one notification per genuine failure; repeated failure is
    self-limiting, because by the third attempt the ≈3-attempts rule has fired
    and the work is being handed over — a moment that *should* be loud.
  - **Nothing depends on a hand-back comment existing.** A draft is a saved
    state as best the last session could manage, and a session cut short may
    never have got to explain itself. The durable context is the **issue**,
    written before the work started and meant to be read cold. Write a comment
    when there is a chance to; never rely on one being there.
- **Branch naming.** A PR that closes an issue uses `{issue_number}-short-slug`
  (e.g. `3-render-pipeline`). An issue-less PR uses a readable short slug of
  its subject. Lowercase-hyphenated, brief.
- **One worktree per issue, and every unblocked issue at once.** Issues that
  block neither each other nor a common third are worked **in parallel, all of
  them** — five unblocked issues means five branches being built right now, not
  the best one of the five — and each gets **its own git worktree** branched
  off the latest `main`: one checkout per branch, never two branches taking
  turns in one. Start as many as the machine can actually carry; if it cannot
  carry them all, priority order decides which go first. The isolation
  is what makes a branch's gates mean anything: a shared checkout mixes
  another issue's edits into `make gates` and thrashes `target/` between
  builds. Each branch runs its own CI as a **signal** that it is healthy; the
  run that **gates** a merge is the one on the rebased state below, so
  re-running the other open PRs after every merge proves nothing — each gets
  its green when its turn to rebase comes.
- **Merging — serialized, one at a time.** Rebase the PR onto the latest
  `main` → CI green on that rebased state → `make mergeable PR=N` → merge →
  repeat, one PR at a time.
  **`make mergeable` is not optional and its answer is not negotiable.** It
  asks GitHub whether a run genuinely happened on the head commit, because
  "the checks look green" and "the checks ran" are different claims, and #153
  is what happens when they diverge: a ready pull request whose only run was a
  skipped one reads as passing and compiled nothing. `gh pr checks` is not a
  substitute — it blends runs, which is how a skipped one hides behind a real
  one.
  Rust is compiled: two PRs can each be green alone yet break `main` together,
  so merging cannot be parallelized. The only exception is a PR touching
  **only** Markdown — CI skips those, so they merge freely. `docs/project-format.md`
  is not one of those: tests parse its examples and check its property table,
  so CI runs for it like any source file. If a PR takes ≈3 fix attempts at the
  same failure, or needs a decision you can't make, mark it draft and hand it
  to the user.
- **Architecture- then infrastructure-first (NOT "make it up as we go").**
  When we find a problem — something that bites or will bite more than once, a
  pattern worth adopting, or a gold-standard practice we should have had — we
  document it and fix it **before** continuing. Architecture and
  infrastructure problems **halt feature work**. Each such fix gets its own
  issue when it carries its own responsibility.
- **Dependencies (not batches).** Record how issues relate using GitHub's
  native **Blocked by / Blocks** relationships, and **sub-issues** when one
  issue is literal groundwork for another. Link two issues when one lays the
  groundwork for the next, makes it meaningfully easier, or would conflict too
  much if done concurrently. There are no rigid batches: **the dependency
  graph is the plan.** Any issue with no open blockers and no stage label is
  fair game.
- **Re-read the board after every merge, not after a batch.** A merge changes
  the graph: whatever the merged issue blocked is fair game the moment it
  lands. So the decision is one merge wide — merge, re-read the board, and
  start **every** issue that is now unblocked, unassigned and free of a stage
  label, each in its own worktree — repeating until the board has none left.
  One merge that unblocks three means three more branches, not the best of the
  three: priority orders what gets **merged**, never what gets **worked**, and
  an unblocked issue left unstarted is capacity going to waste. Choosing a
  batch up front and re-checking only once it is done is what this replaces:
  that batch is already stale by its second issue, and work unblocked by the
  first sits waiting on the rest of it for no reason.
- **Assign the user when you start.** The moment work begins on an issue,
  assign the user to it so it's visibly taken. Unassigned = fair game;
  assigned = in progress by someone.
- **Never file an issue and start it in the same breath** — unless the work is
  a direct consequence of another, already-decided issue. An idea still being
  shaped has to settle before anyone codes it.
- **Gates vs. signals — block on correctness, inform on quality.** A check
  that proves **correctness** — build, test, `clippy -D warnings`, the golden
  renders, the size gate — is a **hard gate**: green-to-merge, no exceptions.
  A check that *audits quality* (coverage, mutation testing, perf tracking) is
  an **informational signal**: surfaced where the agent acts on it (job
  summary or PR comment), never blocking a merge. Don't reach for a hard gate
  where a signal does the job.
- **Run the gates before claiming the work is finished, not after CI says so.**
  In practice that is before marking a pull request **ready for review**, which
  is the moment CI is asked to check anything at all. `make gates` runs every
  gate CI blocks on — format, size, the signal renderers, clippy, docs, tests,
  supply chain, and the desktop app's own workspace — and
  `make help` lists them, so the target list is the answer to "what has to be
  green?" rather than `ci.yml` being read for it. The app gate is the only
  conditional one: `app/` is a separate cargo workspace precisely so a headless
  change never builds a graphics dependency tree, so it runs when the branch
  touches `app/` and is reported as skipped when it does not — never green over
  a check that did not run. `make setup`, once per
  clone, points git at the committed hooks in `.githooks/`; from then on
  `make pre-commit` (formatting and the size gate — no build, well under a
  second) runs before every commit, so a file over the line limit never
  reaches a branch. A deliberate work-in-progress commit bypasses it with
  `git commit --no-verify`. Signals stay opt-in and out of `make gates`:
  `make coverage` and `make mutants` are the two, and running either is never
  part of passing.
- **Size gate:** source files ≤ 300 **lines of code**, test files ≤ 150. Blank
  and comment lines do not count — `missing_docs` is a merge gate and the house
  style is to explain the *why*, so a cap that counted prose put those two rules
  in opposition and split files whose code was never the problem. **Group by
  subfolder, not filename prefix** — a shared prefix on sibling files
  (`draw_*`, `probe_*`) is a subfolder waiting to happen; make it one and drop
  the prefix. Enforced in CI by `tools/lint` — run it yourself with
  `make size`, and the pre-commit hook runs it too. Which cap applies to a
  given path is decided and documented in `tools/lint/src/classify.rs`, and
  nothing is grandfathered: a file over the limit gets split, not excused.
- **Agent velocity is first-class.** Agents drive this repo, often unattended.
  Write code that is readable by design and lean — clear code is cheaper to
  reason about and faster for the next agent to extend. Keep CI fast. This is
  part of **Craft**, not a trade-off against it.
- When working through issues unattended: if in doubt on an issue,
  leave a comment on the issue and continue if possible, rather than stalling
  the night on a chat question. Questions during *planning* conversations are
  asked right away.

## Repo-specific conventions

- **No real provider calls in tests, ever.** Veo and ElevenLabs are mocked
  behind traits; golden-render tests use local fixture media only. A test that
  spends money or needs a network is a bug.
- **Secrets resolve in one order, through one resolver**: the environment
  first (an exported variable, or the gitignored `.env` a checkout keeps,
  documented by `.env.example`), then the per-machine settings file a shipped
  build reads — `crates/providers`' `credentials` module, and
  **docs/credentials.md** for the whole of it. `.env` is the development path
  and stops existing the moment somebody installs a build, which is why there
  is a second place and why there is only one resolver. Never in
  `project.json`, never in code, never in fixtures.
- **All ffmpeg invocations go through `scorsese-render`'s command builder.**
  ffmpeg is an external binary on PATH in dev/CI and is bundled beside the
  binary in shipped builds; that indirection lives in one place. No ad-hoc
  `Command::new("ffmpeg")` anywhere else.
- **Golden-render tests compare frames with tolerance** — never byte-equality of
  encoded output. Encoders are not deterministic across versions and platforms;
  frames are what we control. The harness is `crates/golden`, and
  **docs/golden-renders.md** is the rulebook — including the one that matters:
  re-blessing a reference to make CI green is never legitimate.
- **Documentation an agent acts on is gated like code.** `cargo doc` runs with
  `-D warnings`; `docs/project-format.md`'s JSON examples are parsed as
  projects and its animatable-property table is held to what the code
  publishes; every CLI command and flag must carry help text. Any new
  agent-facing surface inherits the rule — MCP tools first among them. What
  this never proves is that the prose is *true*: it stops the shape from
  drifting silently, and reading is still how correctness gets checked.
- **`project.json` format changes are `architecture`-label work** and require
  a schema version bump plus a migration note. The format is the contract
  between the CLI, the MCP server, the GUI, and every saved project.
- **The lint set is chosen, not inherited.** `[workspace.lints]` in the root
  `Cargo.toml` is the whole policy; every crate takes it with
  `lints.workspace = true`. Because CI denies warnings, **each lint there is a
  merge gate**, so the bar for adding one is the gates-vs-signals rule: it must
  prove correctness or an invariant we have actually stated, never a style
  preference we would wave through. That is why `clippy::pedantic` is not on —
  a gate people route around teaches everyone to route around gates. What is on:
  `unsafe_code = "forbid"` (there is none, and it stays that way),
  `missing_docs` (each `lib.rs` doc is a crate's stated boundary, so the docs
  are architecture), `unreachable_pub` (`pub` nobody can reach is API surface
  nobody meant to add), and `clippy::unwrap_used`, `dbg_macro`, `todo`.
  `clippy.toml` holds `allow-unwrap-in-tests`, because a failed `unwrap` in a
  test *is* the assertion — library code, and shared helpers under
  `tests/common/`, say what they assume with `expect("…")` instead. Adding or
  removing a lint is a change to this rule, so it belongs in its own PR with
  the reason recorded here.
- **Nothing in the codebase is temporary**, except small JSON or log files.
  Anything added must benefit the project long-term or be necessary to its
  development — technically, or as a project.

## Issues, labels & priority

- **Issues come before PRs.** The unit of work is a well-specified issue: the
  **what**, **why it belongs in the project**, and the **roadmap — not the
  implementation intrinsics**. A good issue makes clear what the idea is:
  a future Claude reads it cold and says *"I understand the assignment, i know
  how to proceed."* That's what lets an issue run unattended, even overnight.
- **File what you notice.** Claude may open an issue autonomously — for
  anything that will be a recurring theme or problem, or when it realises a
  tool would be useful more than once. Only things whose benefit outweighs the
  cost of implementing them get an issue. If a Claude-written issue is a
  breaking change, changes human-interfacing features, or needs a human's
  judgement call, it must carry one of `idea`, `planning` or `human`. Filing is
  Claude's; deciding is not — the rule against starting an issue in the same
  breath as filing it holds here without exception.
- **Priority by label:** **architecture → infrastructure → bug → foundation →
  feature.** If the way we build isn't solid — a structural shape or
  convention missing (**architecture**), a tool or guardrail missing
  (**infrastructure**), or something broken (**bug**) — we halt and fix that
  first. Then **foundation** work makes the editor itself more complete. Then
  **feature** work serves Claude, the user or the video being made with it.
  **documentation** can be done at any time and never waits its turn.

### Labels

Stage labels (at most one; absence means ready):

- **idea** — might not add value; parked until the user decides. Must **NOT**
  be started.
- **planning** — has value, but the architecture/GUI approach is still being
  discussed. Must **NOT** be started.
- *(no stage label)* — ready: anyone (human or Claude) can tell a Claude agent:
  "do issue N" and Claude can read it, implement it and merge it

Type labels (combinable with a stage label):

- **architecture** — the project's communication structure, conventions,
  `project.json` format changes, crate boundaries.
- **infrastructure** — tools and guardrails for the development process: CI,
  the golden-render harness, gates.
- **bug** — something isn't working.
- **documentation** — edit/add documentation; never waits its turn.
- **feature** — a new editor capability serving the videos made with it.
- **foundation** — groundworks that make the editor itself more complete.
- **human** — AI can't do this end-to-end; needs a human in the loop.

If a `planning` issue would affect how another issue gets implemented or is
thought of, that other issue must be marked **blocked by** the planning issue.
We prioritize anything that accelerates the improvement of the project itself.
Increasing derivatives takes precedence over increasing the variables they aim at.

## Overrides

Any rule in this file may be overridden by the user's explicit say-so — in the
current prompt or a previous one. The **one exception**: an issue carrying a
planning-stage label (`idea` or `planning`) must never be started while the
label is on it. The user may tell you to **remove the label and then do it** —
never to do it with the label still on. (The user *may* greenlight an issue
that is blocked by another; doing so lifts that block.)
