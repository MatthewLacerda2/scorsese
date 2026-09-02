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
- Crate boundaries live in each crate's `lib.rs` module doc — read them before
  adding a dependency between crates.

**Three skills carry the working protocols** — `issue-write`, `issue-batch` and
`ci-merge` — so this file can hold the reasoning and they can hold the steps.
Each one names its own scope and when to reach for it, so this file does not
restate them; invoke them rather than reconstructing a procedure from memory, and
name them when briefing a subagent.

Where a rule below is stated in one line and a skill has ten, the line is the
rule and the skill is how to keep it. Where they disagree, this file wins and the
skill is wrong.

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

**Where this is developed.** One machine, one person: a Ryzen 5 3400G — 4
cores, 8 threads — with 16 GB of RAM and an RTX 2060, on Arch Linux. That is
a current fact and not a decided invariant; the day there are other
contributors it is up for review. Until then it is the premise several rules
below are shaped by, and it is written down so nobody re-derives the industry
default of many contributors on many cold machines and proposes the tooling
that goes with it.

- 8 threads and 16 GB are what "as many as the machine can actually carry"
  means below: **worktrees are cheap, simultaneous builds are not.** Several
  branches checked out costs disk; several concurrent `make gates` runs costs
  more memory than there is, and rustc's linking is where it runs out.
  Parallelise the work, stagger the compiles.
- **One worktree, one `target/`. Never a shared `CARGO_TARGET_DIR`.** Cargo
  keys build artifacts by package, version, features and profile — never by
  source path — so worktrees pointed at one target directory overwrite each
  other's output for every crate a given build did not itself rebuild. The
  phantom failures that causes waste an hour; the false *green* is the reason
  this is a rule, because it says the gates passed on code that was never
  compiled, which is exactly the claim a **ready** pull request makes.
  Cargo's default is already correct — unset, every worktree gets its own
  `target/` — so the rule is to stop overriding it, not to configure anything,
  and `make gates` refuses to run under an override rather than trusting
  anyone to remember. This is unrelated to staggering the compiles above:
  that one is about memory during linking, this one about artifacts colliding.
- **Merging stays serialized; the *waiting* is what may be automated.** Two pull
  requests can each be green alone and break `main` together, because Rust
  type-checks and links across crate boundaries — a changed signature in one
  crate and a new caller in another compile apart and not together. That is the
  compiler's doing and no tooling repeals it, so **speculative CI and parallel
  merging stay out**: they answer a question this repo does not have.
  What *was* wrong is the reason this rule used to give — "at one contributor,
  rebase-and-verify is a minute by hand". The rebase is a minute; the verify is a
  **cold CI run**, and it is the verify that serialises. A batch of 31 issues on
  2026-08-29 paid that twenty times, twice over on one branch whose code did not
  change between attempts. At a branch a week that is invisible; at fifteen in a
  night it is most of the wall clock. So automating **who does the waiting** is
  legitimate work (#491, #492) — under three constraints that are not negotiable:
  a conflict is never resolved by a machine that cannot say *why* the code is
  shaped as it is, **local green is never CI green** (see *CI is a different
  computer* below — it is why golden renders compare with tolerance), and the
  mutation signal never becomes a precondition, because a gate people route
  around teaches everyone to route around gates.
  That work is **`make queue`**, and `ci-merge` has when to reach for it. The
  cheaper answer it was weighed against — *skip the run when the rebase changed
  nothing that matters* — turns out not to exist: a run's verdict is about a
  **tree**, and it carries only to the same tree. Anything weaker than that is
  speculative merging under a friendlier name, and measured over the batch above
  the sound rule would have skipped **none** of its twenty-three re-runs. So the
  ten minutes are real and the only thing to take off the critical path is who
  spends them.
- **A warm `target/` is the fast path.** Cross-machine compilation caches
  (`sccache` and the like) buy cold-build speed by turning off cargo's
  incremental compilation, which is the wrong trade on a machine that is
  always warm.
- **The GPU is real and CI has none.** The compositor is CPU tiny-skia first
  regardless — that is settled architecture, not a consequence of hardware.
  The consequence to hold onto is narrower: anything GPU-dependent can be
  built and tried here, but **can never be a merge gate.**
- **CI is a different computer.** GitHub-hosted `ubuntu-24.04`: cold, no GPU,
  and — Arch being a rolling release — very often a different ffmpeg build
  than the one that produced a frame locally. "Works here" and "passes CI"
  are separate claims, which is why golden renders compare frames with
  tolerance rather than encoded bytes.

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
  **Issue-less PRs are allowed only** for documentation updates or bug fixes.
  Either way the PR description still has to clear the three gates.
  **The `issue-write` skill** has what an issue must contain and which label it
  carries; **`issue-batch`** has how a set of them is worked; **`ci-merge`** has
  how a branch gets from finished to merged. Invoke them rather than
  reconstructing the steps, and tell a subagent to invoke them too.
- **A ready pull request claims it passes; a draft makes no such claim.** CI runs
  on ready pull requests and on `main`, nowhere else, so a red run always means a
  claim was broken — worth a notification every time. A draft is not decoration
  on unfinished work; it is **how work survives** a session that ends badly, and
  the durable context is the **issue**, written to be read cold. Never rely on a
  hand-back comment existing. A red ready pull request **stays ready** and is
  fixed forward.
- **Merging is serialized, one branch at a time**, because Rust is compiled: two
  pull requests can each be green alone and break `main` together. The only
  exception is a PR touching **only** Markdown, which CI skips.
  **`make mergeable` is not optional and its answer is not negotiable** — it asks
  whether a run genuinely happened on the head commit, because "the checks look
  green" and "the checks ran" are different claims, and #153 is what happens when
  they diverge. `gh pr checks` is not a substitute; it blends runs.
- **Coding parallelises; merging does not**, so the merge queue is the
  bottleneck and every branch behind another pays a rebase per merge ahead of it.
  Two branches in flight — one merging, one being written — is the shape that
  keeps a queue moving without paying for it twice. The binding constraint is
  **file collision**, not branch count. `issue-batch` has the arithmetic.
- **One worktree per branch**, off the latest `main`, removed the moment it
  merges. The isolation is what makes a branch's gates mean anything, and each
  worktree carries a full `target/` measured in gigabytes.
- **Architecture- then infrastructure-first (NOT "make it up as we go").**
  When we find a problem — something that bites or will bite more than once, a
  pattern worth adopting, or a gold-standard practice we should have had — we
  document it and fix it **before** continuing. Architecture and
  infrastructure problems **halt feature work**. Each such fix gets its own
  issue when it carries its own responsibility.
- **The dependency graph is the plan.** Record how issues relate with GitHub's
  **Blocked by / Blocks** and **sub-issues**; there are no rigid batches. Split
  by responsibility, never by parallelism — sub-issues that all touch the same
  type are one branch.
- **A stage label is the only thing that stops an issue being started.**
  `planning` and `human` mean *not yet*, and they are absolute. Absent one, an
  issue is startable the moment it exists, including one Claude filed a minute
  ago. The judgement lives in the label; asking the question a second time at
  the moment work begins adds nothing.
- **Gates vs. signals — block on correctness, inform on quality.** A check
  that proves **correctness** — build, test, `clippy -D warnings`, the golden
  renders, the size gate — is a **hard gate**: green-to-merge, no exceptions.
  A check that *audits quality* (coverage, mutation testing, perf tracking) is
  an **informational signal**: surfaced where the agent acts on it (job
  summary or PR comment), never blocking a merge. Don't reach for a hard gate
  where a signal does the job.
- **Run the gates before claiming the work is finished, not after CI says so.**
  In practice that is before marking a pull request **ready for review**, which is
  the moment CI is asked to check anything at all. `make gates` runs every gate CI
  blocks on and `make help` lists them, so the target list — not `ci.yml` — is the
  answer to "what has to be green?". The app gate is the only conditional one and
  reports **skipped** when a branch touches nothing under `app/`; skipped is never
  green over a check that did not run. `make setup`, once per clone, points git at
  the committed hooks; from then on `make pre-commit` — formatting and the size
  gate, no build — runs before every commit, so an oversized file never reaches a
  branch. `git commit --no-verify` bypasses it for a deliberate work-in-progress.
- **Signals stay opt-in and out of `make gates`:** `make coverage` and
  `make mutants`. Running either is never part of passing, and **a signal never
  holds a merge** — that is what makes it a signal. Read the mutation report when
  it lists survivors in code **this branch wrote**; a report with nothing in it,
  or whose survivors sit in untouched code, needs no reading at all. The exits are
  **fix it**, **exclude it with a written reason**, or **file it as its own
  issue** — there is no fourth, and none of them blocks the queue. The
  **`ci-merge` skill** has how to sort them and how to triage a survivor;
  `docs/mutation-testing.md` has what the report deliberately does not list.
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
- **Which model does what — a hint, not a rule.** Design, implementation,
  triage and anything needing a judgement call want the strongest model
  available; a rebase, a merge conflict in a module list, moving an attribute
  between files and other mechanical work do not. Most of an agent's sessions on
  a branch are the second kind, and paying top rate for them is where a night's
  budget quietly goes. A hint because the line is not crisp — a "mechanical"
  rebase that turns out to need two authors' prose reconciled is not mechanical —
  so whoever spawns the work calls it, and gets it wrong upwards.

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
  a schema version bump. The format is the contract between the CLI, the MCP
  server and the GUI — the contract *now*, not across time.
- **A change to what a recipe renders to requires a `SYNTH_VERSION` bump**, in
  the same commit, for the same reason a format change requires a
  `schema_version` bump: breaking loudly is the point. A bake in `generated/`
  is addressed by a hash of the recipe **and** that number, so bumping it is
  what makes every affected file miss the cache and be re-rendered — and not
  bumping it leaves every project on disk holding audio its own recipe no
  longer describes, silently. The number is declared rather than derived
  because deriving it means hashing rendered output, and a digest has no
  tolerance to spend on a platform's `sin` and `exp` differing; the constant's
  own doc in `crates/zimmer/src/lib.rs` carries the whole argument. So: touch a
  source, a filter, an envelope or an effect in `zimmer` and the samples move —
  bump it. Touch only prose, a name or a document type — leave it alone.
  **Verify by rendering only when the change touches rendering maths.** Baking a
  probe corpus against two checkouts settles a genuine doubt — an edited note
  loop, a shared helper moved, a stage reordered — and is waste when the diff
  already answers it: a new optional field defaulting to old behaviour cannot
  move an existing recipe's bytes. Say so in one line and move on. The
  constant's own doc records what previous branches checked, and why.
- **There is no backwards compatibility, and that is the policy until the user
  says otherwise.** Nothing is kept working for the sake of a `project.json`
  saved by an older build: no migration notes, no reading an older
  `schema_version`, no field kept alive because something might still write it.
  There is one machine and one person, and no saved project anybody would mind
  losing — so compatibility written now is weight carried on behalf of a user
  who does not exist, and it is the kind of weight that makes every format
  change expensive enough to argue about, which is how a format ossifies while
  it is still wrong. **The version bump is not compatibility work and it stays
  mandatory**: `Project::load` refuses a document whose `schema_version` is not
  this build's, so bumping is exactly what turns a silent reinterpretation into
  a loud refusal. Breaking loudly is the point. The day somebody has a project
  they cannot afford to lose, this rule is the one to revisit — and it is the
  user's call, not a thing to infer from a project having got big.
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
  **what**, **why it belongs**, and the **roadmap — not the implementation
  intrinsics**. A future Claude reads it cold and says *"I understand the
  assignment, I know how to proceed."* That is what lets an issue run unattended,
  even overnight.
- **File what you notice.** Claude may open an issue autonomously — for anything
  that will recur, or when a tool would be useful more than once — provided the
  benefit outweighs the cost of building it. The strongest issues come out of
  doing the work.
- **A bug is always filable.** Claude may open a `bug` issue autonomously the
  moment it spots one — the test above is about whether something is worth
  *building*, and never about whether a defect is worth *recording*. If the bug
  questions a decision or surfaces a foundational problem, tell the user, because
  that is a judgement call. Otherwise keep the description brief and carry on.
- **Priority by label:** **architecture → infrastructure → bug → foundation →
  feature.** If the way we build isn't solid — a structural shape or convention
  missing (**architecture**), a tool or guardrail missing (**infrastructure**), or
  something broken (**bug**) — we halt and fix that first. Then **foundation**
  work makes the editor itself more complete. Then **feature** work serves Claude,
  the user or the video being made with it. **documentation** can be done at any
  time and never waits its turn. Priority orders what gets **merged**, never what
  gets **worked**.
- **Stage labels — at most one, and absence means ready.** `planning` (nobody
  has decided this is worth doing, or the approach is not settled), `human`
  (needs a human end-to-end). Both mean **do not start**. A Claude-written issue
  must carry one if it is a breaking change,
  changes human-facing behaviour, needs a judgement call, or proposes a structural
  change. A `bug` usually should not — the deciding already happened when the code
  broke.
- **The `issue-write` skill** has the rest: what each type label means, what a
  good issue body contains, how relationships are recorded, and the three gates in
  full.

## Overrides

Any rule in this file may be overridden by the user's explicit say-so — in the
current prompt or a previous one. The **one exception**: an issue tagged
`planning` must never be started while the label is on it. The user may tell
you to **remove the label and then do it** —
never to do it with the label still on. (The user *may* greenlight an issue
that is blocked by another; doing so lifts that block.)
