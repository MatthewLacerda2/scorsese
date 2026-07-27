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

## Start here

- **docs/project-format.md** — the `project.json` schema: assets, tracks,
  clips, keyframes, paths, and what validation checks.
- **docs/golden-renders.md** — the pixel gate: what a fixture is, how frames are
  compared, and when re-blessing a reference is legitimate. Read it before
  changing anything a render's output depends on.
- **docs/output-formats.md** — the containers and codecs a render delivers in,
  which combinations are refused, and why that list is deliberately short.
- Crate boundaries live in each crate's `lib.rs` module doc — read them before
  adding a dependency between crates.

## Architecture — decided, do not redesign

These decisions are settled. Changing one is `architecture`-label work, not a
side effect of a feature PR.

- **A project is a directory** (`*.scor/`): `project.json` + `assets/`
  (imported media, copied on import) + `generated/` (provider outputs,
  content-addressed by prompt hash) + `cache/` (rebuildable, gitignored). All
  paths inside `project.json` are relative to the project root. **No absolute
  paths, ever** — a project must survive `scp -r` between machines.
- **Assets are entities, clips are references.** `project.json` has an assets
  table (id, kind, path, sha256 hash, probed metadata); tracks hold clips that
  reference assets **by id — never by path**. Asset kinds: `video`, `image`,
  `audio`, `text`, `generated_video` (Veo prompt), `generated_audio`
  (ElevenLabs TTS prompt).
- **Prompt clips and the sketch lifecycle.** A `generated_*` asset carries a
  prompt and a state: `sketch → queued → generated → stale` (stale = prompt
  edited after generation). Sketch/stale clips render as slug cards (prompt
  text on a gray card) so a full preview cut costs $0. "GO" generates only
  sketch/stale assets. Generated files are cached by prompt hash and never
  regenerated for an unchanged prompt.
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

### Crate map

`crates/core` (model, serde format, validation) ← `crates/compositor` (frame
rendering, CPU tiny-skia first) ← `crates/render` (ffmpeg orchestration) ;
`crates/providers` (Veo + ElevenLabs, prompt-hash cache) ; `crates/cli` (the
headless `scorsese` binary) ; `crates/mcp` (MCP server, thin wrapper over the
same logic) ; `crates/golden` (test infrastructure: the golden-render gate,
which nothing ships and nothing depends on) ; `app/` (Tauri GUI, not in the
workspace yet). Each `lib.rs` doc states what its crate must never depend on —
those boundaries are enforced in review.

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
- **Branch naming.** A PR that closes an issue uses `{issue_number}-short-slug`
  (e.g. `3-render-pipeline`). An issue-less PR uses a readable short slug of
  its subject. Lowercase-hyphenated, brief.
- **Merging — serialized, one at a time.** Rebase the PR onto the latest
  `main` → CI green on that rebased state → merge → repeat, one PR at a time.
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
- **Run the gates before the push, not after it.** `make gates` runs every
  gate CI blocks on — format, size, the signal renderers, clippy, docs, tests,
  supply chain — and
  `make help` lists them, so the target list is the answer to "what has to be
  green?" rather than `ci.yml` being read for it. `make setup`, once per
  clone, points git at the committed hooks in `.githooks/`; from then on
  `make pre-commit` (formatting and the size gate — no build, well under a
  second) runs before every commit, so a file over the line limit never
  reaches a branch. A deliberate work-in-progress commit bypasses it with
  `git commit --no-verify`. Signals stay opt-in and out of `make gates`:
  `make coverage` and `make mutants` are the two, and running either is never
  part of passing.
- **Size gate:** source files ≤ 300 lines, test files ≤ 150. **Group by
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
- When working through issue batches unattended: if in doubt on an issue,
  leave a comment on the issue and continue if possible, rather than stalling
  the night on a chat question. Questions during *planning* conversations are
  asked right away.

## Repo-specific conventions

- **No real provider calls in tests, ever.** Veo and ElevenLabs are mocked
  behind traits; golden-render tests use local fixture media only. A test that
  spends money or needs a network is a bug.
- **Secrets via `.env`** (gitignored), documented in the committed
  `.env.example`. Never in `project.json`, never in code, never in fixtures.
- **All ffmpeg invocations go through `scorsese-render`'s command builder.**
  ffmpeg is an external binary on PATH in dev/CI and a bundled Tauri sidecar
  in shipped builds; that indirection lives in one place. No ad-hoc
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
