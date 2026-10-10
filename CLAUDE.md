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

**The user never deals with implementation.** In the app or on the web, the
whole interface is: *send me your files and prompt the video into existence.*
Project JSON, recipes, HTML pages, codecs, font licences, which provider does
what — those are how Claude does the work, never something the user is asked
to understand or decide. When a feature would put a technical choice in front
of the user, it ships with a default good enough that they never have to touch
it, and Claude explains it only if asked. The one thing always put to the user
is **money**: the quote-and-ask before a generation stays.

### The app is the product; the web app is a convenience layer

Scorsese is open source. Anyone can clone it, build it, bring their own API
keys and have all of it for free — the desktop app, the CLI and the MCP server
on local `.scor` folders. **That is the product**, and nothing it can do is held
back for the web.

The hosted web app (#527) is a **convenience layer** over it, for people who
would rather not install a Rust toolchain or learn what an API key is: a URL, a
login, a library of their own files and an assistant that edits for them. They
pay for not having to set anything up. It is a thin client of the same `core`,
`render` and `providers`, and it **must be able to do everything the app does**
— it follows the app, never leads it. An editing capability the app has and the
web does not is a gap on the web side, to be filed and closed; an editing
capability only the web has is a design error. (Accounts, credits and the
library are web-only by nature — they are the convenience, not the editing.)

Where the two differ it is because of **whose machine it is**. Locally the
machine is the user's, and so are its risks and settings. The web app runs on
the maintainer's machine for other people, so anything that executes a user's
content there is isolated and offline, with no opt-out (#594).

**Who each surface is for** (the maintainer, 2026-10-03). This decides how much
a surface explains and how much it does for its user:

- **CLI and MCP: the technical user, and the main user of the product.** This is
  someone who drives scorsese from their own agent (Claude Code, Codex,
  Antigravity…) and is expected to manage on their own everything the web takes
  care of: what an API key is and how to set one, which model their client runs,
  what a cache miss costs. That's why the local MCP has no model picker and no
  hand-holding warnings, and why its agent is their own rather than a built-in
  chatbot.
- **The desktop app: for previewing and the plainest operations.** It's a
  timeline, cropping, move/rotate/scale, and scrubbing to see the result. Its
  user is still the CLI/MCP user, watching what their agent built. It has no
  built-in assistant, and gets none.
- **The web app: convenience, as above.** No keys, no setup, a built-in
  assistant and plain words. Whatever the CLI user is expected to know, the web
  either handles for its user or explains in one line.

The north star does not change, it widens: *the user* includes people who are
not the maintainer, on either side. **docs/web.md** has the web side's settled
shape.

## Start here

- **docs/project-format.md** — the `project.json` schema: assets, tracks,
  clips, keyframes, paths, and what validation checks.
- **docs/recipes.md** — the synthesis recipe format: what to write in
  `recipes/*.json` to get an effect or a score out of `scorsese synth`. Free,
  offline, deterministic — read it before reaching for a sound file.
- **docs/references.md** — what five real records measure, and which one to
  hold a cue against: the floor a score must clear, never the definition of
  success. Read it before writing a song recipe and again after its bake.
- **docs/pages.md** — how to write a web page the timeline plays: the contract
  it is drawn under, what is there offline, and worked pages. Read it before
  writing one.
- **docs/prompts.md** — the other brief: what a provider actually does with
  certain words, each entry learned by paying for a generation. Read it before
  writing a prompt, because being wrong about one is not free.
- **docs/prices.md** — what "not free" comes to: the provider rate tables, how
  they are kept honest, and why a cost is only ever an estimate. Read it before
  quoting a number at the user.
- **docs/golden-renders.md** — the pixel gate: what a fixture is, how frames are
  compared, and when re-blessing a reference is legitimate. Read it before
  changing anything a render's output depends on.
- **docs/output-formats.md** — the containers and codecs a render delivers in,
  which combinations are refused, and why that list is deliberately short.
- **docs/credentials.md** — where a provider key comes from, in one order for
  every way scorsese is run, and the spending ceiling that lives beside it.
- **docs/mcp.md** — the tools `scorsese-mcp` exposes, how a client is pointed
  at it, and the rule that every tool and every argument describes itself.
- **docs/web.md** — the hosted web app: its containers, per-user isolation,
  why the edit is a JSON document while everything around it is tables, and
  money as integer micro-dollars. Read it before touching the server or `web/`.
- **web/README.md** — the React front-end: running it, its dev proxy to the
  server, its gates (`make web`, and `make web-e2e` for the end-to-end flows)
  and which kind of test a new one is.
- Crate boundaries live in each crate's `lib.rs` module doc — read them before
  adding a dependency between crates.

**Three skills carry the working protocols** — `issue-write`, `issue-batch` and
`ci-merge` — so this file can hold the reasoning and they can hold the steps.
Each one names its own scope and when to reach for it, so this file does not
restate them; invoke them rather than reconstructing a procedure from memory, and
name them when briefing a subagent.

Where a rule below is stated in one line and a skill has ten, the line is the
rule and the skill is how to keep it. The two are written to agree: a
disagreement between them is a bug, fixed in the change that finds it.

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
- **New motion graphics are pages; the native graphics are frozen** (the
  maintainer, 2026-10-06). An html clip (#594, `docs/pages.md`) can do what
  native shapes, icons and text animation do, and more. The 2026-10-06 video
  was built from twelve pages and no native graphics, where the 2026-10-01 one
  used 128 shapes. So **native `text` and `color` stay as the fast path**:
  instant to preview, one tool call, working on every platform and on the web.
  **`shape`, `icon`, arrows with `attach` / `follow`, text `reveal` / `number`
  and gradients are frozen**: bugs are fixed, nothing is added. A new graphics
  capability (a text shadow, a new gradient, a title preset) goes to pages and
  their motion kit (#812), never to the native kinds. Nothing is removed yet:
  pages cannot be captured on the web (#778) or on Windows (#797) and preview
  slowly (#809), and removing a kind is a format change with a migration.
  Revisit once those land and the tool-call record shows no new native shapes
  being made.
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
- **The web app's built-in assistant runs on the model each project picks**
  (#705): Claude Sonnet 5.5 by default, from a dropdown in the chat panel.
  Choosing models for **our own client** is a product decision; the **tool
  surface** it calls still assumes nothing about who is calling.
  `crates/server/CLAUDE.md` has the rest.

### Crate map

`crates/core` (model, serde format, validation) ← `crates/compositor` (frame
rendering, CPU tiny-skia first) ← `crates/render` (ffmpeg orchestration) ;
`crates/zimmer` (synthesis: recipe documents → samples; **no I/O at all**,
and no dependency on `core`) ← `crates/providers` (Veo + ElevenLabs +
synthesis, brief-hash cache) ; `crates/zimmer` ← `crates/render` too, for
reading only — loudness and song surveys, never synthesising ; `crates/cli` (the headless `scorsese` binary) ;
`crates/mcp` (MCP server, thin wrapper over the same logic) ; `crates/golden`
(test infrastructure: the golden-render gate, which nothing ships and nothing
depends on) ; `crates/server` (`scorsese-server`, the web API: HTTP, Postgres,
the job queue — a thin client of `core` / `render` / `providers` exactly like
`cli` and `mcp`, and a dependent of `mcp` itself for its tool registry (#530),
with **no editing logic of its own**; code an endpoint needs
that the CLI does not share belongs a layer down) ; `web/` (the React
front-end — its own Bun project with its own conditional gate, the way `app/`
is its own workspace; it talks to the server over HTTP and to nothing else) ;
`app/` (the egui desktop app — its own cargo workspace, so a
graphics dependency tree never slows `cargo test --workspace`). Each `lib.rs` doc
states what its crate must never depend on — those boundaries are enforced in
review.

**`zimmer` is ours** (the joke told twice: Scorsese directs, Zimmer scores),
and **[rusty](https://github.com/MatthewLacerda2/rusty)**, the owner's game
engine, depends on it too, so its public API is rusty's contract.
`crates/zimmer/CLAUDE.md` has the name, the rusty rules and `SYNTH_VERSION`.

## How we work

The steps live in the skills; what follows is each rule in a line or two, so it
is in front of every session. One person develops this, on their own machine or
in a cloud session; `issue-batch` has what that premise shapes.

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
- **Take the initiative.** Claude can always take the initiative of adding or
  changing things so long as they make scorsese and the videos made with it
  better, or make the development of scorsese itself better — Claude does not
  wait to be told to improve the codebase. The user follows Claude's
  recommendations, and implementing then validating later beats waiting: a
  clear win without a downside is **implemented**, not proposed. The one
  exception is a change to **scorsese's design itself** — the decided
  architecture above, the `project.json` format, the shape of the tool surface,
  the conventions in this file — where Claude proposes and the user decides,
  unless one option is a plain win-win, which Claude takes. Initiative still
  runs through the normal flow, and nothing that spends the user's money — a
  provider generation, `make live-check` — is done on initiative: it is asked
  first, every time.
- **Flow:** idea → (issue) → branch → PR → CI green → merge. **Nothing is
  committed to `main` directly**: every change, however small, arrives as a
  pull request. An issue is how work is planned, not a toll on every change
  (`issue-write`). A PR that has an issue references the one it closes, and
  every PR's description clears the three gates.
- **One worktree per branch**, off the latest `main`, removed the moment it
  merges, and **never a shared `CARGO_TARGET_DIR`** — it makes worktrees
  overwrite each other's artifacts and produces a false green; `make gates`
  refuses to run under one. Compiles are staggered and sized from the machine
  you are on, measured. One of the operator's machines also hosts the web app,
  so a heavy build there competes with a paying user's render. `issue-batch`
  has all of it, and when to hand branches to cloud sessions.
- **Merging is serialized, one branch at a time**, because Rust is compiled:
  two pull requests can each be green alone and break `main` together. The only
  exception is a PR touching **only** Markdown, which CI skips. What may be
  automated is who does the waiting (`make queue`). **`make mergeable` is not
  optional and its answer is not negotiable**; `gh pr checks` is not a
  substitute. `ci-merge` has why and how.
- **A ready pull request claims it passes; a draft makes no such claim.** CI
  runs on ready pull requests and on `main`, nowhere else. A draft is how work
  survives a session that ends badly; a red ready pull request stays ready and
  is fixed forward.
- **Run the gates before marking a pull request ready**, not after CI says so.
  `make gates` runs every gate CI blocks on and `make help` lists them; the app
  and web gates report **skipped** when a branch touches nothing under `app/` or
  `web/`, and skipped is never green. `make setup`, once per clone, installs the
  pre-commit hook (formatting and the size gate). **CI is a different
  computer** — cold, no GPU, often a different ffmpeg — so local green is never
  CI green, and nothing GPU-dependent is ever a merge gate.
- **Gates vs. signals — block on correctness, inform on quality.** Build, test,
  `clippy -D warnings`, the golden renders and the size gate are **hard
  gates**. Coverage, mutation testing and perf tracking are **signals**: opt-in,
  on a schedule or when asked, never in a pull request's CI run, never holding a
  merge. Ask for a mutation run (`make mutants-remote`) when a branch adds
  mechanism, before marking it ready. `ci-merge` has how to triage the report.
- **Size gate:** source files ≤ 300 **lines of code**, test files ≤ 150; blank
  and comment lines do not count. **Group by subfolder, not filename prefix** —
  a shared prefix on sibling files is a subfolder waiting to happen. Nothing is
  grandfathered: a file over the limit gets split, not excused. `make size`
  runs it; `tools/lint/src/classify.rs` decides which cap applies.
- **Infrastructure- then architecture-first (NOT "make it up as we go").**
  When we find a problem — something that bites or will bite more than once, a
  pattern worth adopting, or a gold-standard practice we should have had — we
  document it and fix it **before** continuing. Infrastructure and
  architecture problems **halt feature work**, and each such fix gets its own
  issue when it carries its own responsibility.
- **Agent velocity is first-class.** Agents drive this repo, often unattended.
  Write code that is readable by design and lean, and keep CI fast. This is part
  of **Craft**, not a trade-off against it. Use the strongest model for
  judgement work and a cheaper one for mechanical work (`issue-batch`).
- **Working unattended, decisions are written down, never waited on**: a
  judgement call an issue left open takes the default the issue, this file or
  Filmora 9 (for taste) points to, and the choice and its reason go on the issue
  or PR. A check only a human can do (a real window, real speakers, taste) is
  **never a merge hold**. Questions during *planning* conversations are asked
  right away. `issue-batch` has the rest.

## Repo-specific conventions

- **No real provider calls in tests, ever.** Veo and ElevenLabs are mocked
  behind traits; golden-render tests use local fixture media only. A test that
  spends money or needs a network is a bug.
- **Secrets resolve in one order, through one resolver** (`crates/providers`'
  `credentials` module, **docs/credentials.md**): the environment first (an
  exported variable or the gitignored `.env`), then the per-machine settings
  file a shipped build reads. Never in `project.json`, never in code, never in
  fixtures.
- **All ffmpeg invocations go through `scorsese-render`'s command builder.**
  ffmpeg is on PATH in dev/CI and bundled beside the binary in shipped builds;
  that indirection lives in one place. No ad-hoc `Command::new("ffmpeg")`
  anywhere else.
- **Golden-render tests compare frames with tolerance**, never byte-equality of
  encoded output. **docs/golden-renders.md** is the rulebook — including the
  one that matters: re-blessing a reference to make CI green is never
  legitimate.
- **Documentation an agent acts on is gated like code.** `cargo doc` runs with
  `-D warnings`; `docs/project-format.md`'s JSON examples are parsed as
  projects and its animatable-property table is held to what the code
  publishes; every CLI command and flag must carry help text. Any new
  agent-facing surface inherits the rule — MCP tools first among them. What
  this never proves is that the prose is *true*: reading is still how
  correctness gets checked.
- **`project.json` format changes are `architecture`-label work** and require
  a schema version bump; **`crates/core/CLAUDE.md`** has what the bump carries.
- **The lint set is chosen, not inherited.** `[workspace.lints]` in the root
  `Cargo.toml` is the whole policy, and each lint there is a merge gate; the
  comment above it has the bar for adding one.
- **Nested `CLAUDE.md` files** hold what only matters inside one folder
  (`crates/zimmer`, `crates/core`, `crates/server`). They load when a file
  there is read, so a rule needed before that stays here.
- **Nothing in the codebase is temporary**, except small JSON or log files.
  Anything added must benefit the project long-term or be necessary to its
  development — technically, or as a project.

## Issues, labels & priority

`issue-write` has the whole of this: what an issue contains, the labels, how
relationships are recorded. The rules every session needs:

- **Issues are how work is planned.** A future Claude reads one cold and says
  *"I understand the assignment, I know how to proceed."* The **what**, **why it
  belongs**, and the **roadmap — not the implementation intrinsics**.
- **File what you notice — it is a duty, not an option** (the user,
  2026-10-04, #757). Whatever Claude finds — a bug, a missing feature, a gap in
  the design, a quality-of-life improvement — is folded into the task at hand
  or filed as an issue, never dropped. **A change that needs the user's
  approval is never folded in**: one that changes how the user sees or
  understands their existing data or project, changes stored data, or needs a
  migration. **Every issue Claude writes on its own carries the `agent`
  label.**
  - **While a video is being made, the findings wait for the debrief** (the
    user, 2026-10-10). Making a real video is the hardest test scorsese gets
    and where most findings come from: bugs, missing tools, papercuts, things
    learned about where it is posted (Instagram's recommended loudness). But
    the session is about the video, so during the edit Claude files nothing,
    opens no branch and does not stop to discuss them; it writes each one down
    as it is noticed (a scratch file named for the project, so a compaction
    does not lose it) and keeps editing. The debrief comes **after**: when the
    user says the video is done, or when Claude has rendered and delivered the
    video or audio and considers it finished. Then every finding on the list is
    filed or folded under the rules above, and the user is told what went
    where. The one exception is a bug that **stops the video being made**:
    worked around if it can be, fixed then if it cannot, because otherwise
    there is no video. This is for someone working in a clone of the repo
    (CLI, MCP, the desktop app). **The web app's assistant never does it**:
    its user is making a video, not developing scorsese.
- **Priority by label:** **infrastructure → architecture → bug → foundation →
  feature.** A bug in the development tooling itself (CI, the gates, the hooks,
  `make queue` / `make mergeable`) ranks as infrastructure. **documentation**
  never waits its turn. Priority orders what gets **merged**, never what gets
  **worked**.
- **Stage labels — at most one, and absence means ready.** `planning` (the user
  has not finished agreeing on a design) and `human` (needs a human end-to-end)
  both mean **do not start**, and they are the only thing that does.
- **Stage labels are the user's to ask for; Claude settles the design instead**
  (the user, 2026-10-07). An issue that needs a decision is not parked under a
  label: Claude states the decisions with a recommendation each, the user
  answers, and the answers go into the issue, filed **startable**. **The one
  exception Claude applies itself:** a finding made **unattended** that needs
  the user's approval is filed with `planning`, and raised at the next
  check-in.
- **The dependency graph is the plan.** Blocked by / Blocks and sub-issues;
  split by responsibility, never by parallelism.

## Overrides

Any rule in this file may be overridden by the user's explicit say-so — in the
current prompt or a previous one. The **one exception**: an issue tagged
`planning` must never be started while the label is on it. The user may tell
you to **remove the label and then do it** —
never to do it with the label still on. (The user *may* greenlight an issue
that is blocked by another; doing so lifts that block.)
