# CLAUDE.md

**scorsese** is a cross-platform video editor built for agentic workflows: a
human or a Claude agent assembles a video from a JSON project file and renders
it headlessly, and a GUI lets humans scrub, tweak and review.

Use plain language with the user and explain things at a high level; go into
Rust, ffmpeg or the compositor only when it is needed. The user does not read
the code: never point them at a file or a line of it (naming a crate, a module
or a dependency is fine). Issues and PRs can be technical and detailed: they
are documentation for a future Claude. Tell the user when something is
bottlenecking you. Don't recite this file unless a rule blocks you or explains
a conclusion.

## North star

Scorsese must let the user fully realise the video they envision, in the
fastest way possible, and Claude must be able to carry the whole editing
process from the user's prompts and files alone. The GUI is only where the user
watches the plan and the edit. Every change to the codebase must serve this.

### What scorsese is, and is not

A tool for **creating and editing videos**: cuts, titles, music, narration,
pacing. **Not** a compositing suite: no node graphs, tracking, rotoscoping,
colour pipelines or VFX.

The word is **approachable**: what a person with an idea and some footage would
reach for is in; what only makes sense to a professional editor is probably
out. It cuts both ways: a reason to refuse an elaborate feature, and to build
an obvious one well.

**Filmora 9 is the reference for taste**, not a specification: its built-in
animations and fonts, its audio controls (volume as plain linear ramps between
points), speed changes on video and audio clips. Ignore what exists because a
professional demanded it.

**The user never deals with implementation.** In the app or on the web, the
whole interface is *send me your files and prompt the video into existence*.
Project JSON, recipes, pages, codecs, font licences and providers are how
Claude does the work, never a choice put to the user: a feature that would put
one there ships with a default good enough never to touch, and Claude explains
it only if asked. The one thing always put to the user is **money**: the
quote-and-ask before a generation stays.

**Stock before generation; narration freely.** Prefer Pixabay stock to
generated content and generate what must be custom-made (**docs/stock.md**).
ElevenLabs is inexpensive: use it freely, never without a reason.

### The app is the product; the web app is a convenience layer

Scorsese is open source: anyone can build the desktop app, the CLI and the MCP
server and use them on local `.scor` folders with their own keys, for free.
**That is the product**, and nothing it can do is held back for the web.

The hosted web app is a **convenience layer** over the same `core`, `render`
and `providers`, for people who would rather not install a toolchain or learn
what an API key is. It **must do everything the app does** and follows it,
never leads: an editing capability only the app has is a gap to file; one only
the web has is a design error. Accounts, credits and the library are web-only
by nature. On the web, anything executing a user's content is isolated and
offline, with no opt-out.

**Who each surface is for**, which decides how much it explains and does:

- **CLI and MCP: the technical user, and the main user of the product.** They
  drive scorsese from their own agent and manage keys, models and costs
  themselves: no model picker, no hand-holding warnings, no built-in chatbot.
- **The desktop app: previewing and the plainest operations** (a timeline,
  crop, move/rotate/scale, scrubbing) for someone watching what their agent
  built. It has no built-in assistant, and gets none.
- **The web app: convenience.** No keys, no setup, a built-in assistant and
  plain words: whatever the CLI user is expected to know, the web handles or
  explains in one line. **docs/web.md** has its settled shape.

## Start here

- **docs/project-format.md**: the `project.json` schema and its validation.
- **docs/recipes.md**: synthesis recipes; before reaching for a sound file.
- **docs/references.md**: what real records measure; before and after a bake.
- **docs/pages.md**: pages the timeline plays; before writing one.
- **docs/styles.md**: how each kind of video is made; before laying one out.
- **docs/prompts.md**: what providers do with words; before writing a prompt.
- **docs/prices.md**: the rate tables; before quoting a cost.
- **docs/golden-renders.md**: the pixel gate; before changing a render.
- **docs/output-formats.md**: the containers and codecs a render delivers.
- **docs/credentials.md**: provider keys and the spending ceiling.
- **docs/mcp.md**: the MCP tools and pointing a client at them.
- **docs/stock.md**: free stock (Pixabay, LottieFiles) and when to search it.
- **docs/live-check.md**: `make live-check`, real calls to every vendor.
- **docs/mutation-testing.md**: what the mutation signal measures.
- **docs/web.md**: the hosted web app; before touching the server or `web/`.
- **web/README.md**: the React front-end, its gates and its kinds of test.
- **each crate's `lib.rs` doc**: its boundaries; before adding a dependency.

**Skills carry the working protocols**: `issue-batch`, `ci-merge`,
`independent-review`, and `post-edit-feedback` once a video is delivered.
Invoke them rather than rebuild a procedure from memory, name them when
briefing a subagent, and fix a disagreement between them and this file where it
is found.

## Architecture — decided, do not redesign

Changing one of these is `architecture`-label work, never a side effect of a
feature PR. So is a `project.json` format change, which bumps the schema
version with a migration (`crates/core/CLAUDE.md`).

- **A project is a directory** (`*.scor/`): `project.json`, `assets/` (imported
  media, copied on import), `generated/` (provider and synthesis output,
  addressed by the hash of its brief), `recipes/` (authored synthesis
  documents: not rebuildable, deleting one loses work) and `cache/`
  (rebuildable, gitignored). Paths in `project.json` are relative to the
  project root, **never absolute**: a project must survive `scp -r`.
- **Assets are entities, clips are references.** The assets table holds id,
  kind, path, sha256 and probed metadata; clips reference assets **by id, never
  by path**. The kinds are `AssetKind`: imported, authored, prompted and
  synthesised.
- **Generated assets have a lifecycle**: `sketch → queued → generated → stale`
  Sketch and stale clips render as slug cards, so a preview cut costs $0. "GO"
  realises only those, and output is cached by its brief's hash, never redone
  for an unchanged brief.
- **What was paid for is kept as the vendor sent it**: byte for byte and never
  overwritten. Anything done to it is a derived copy or a clip-level edit; a
  regenerated brief writes a new file beside the old one.
- **Two kinds of brief.** A `prompt` is a sentence handed to a provider: it
  costs money, needs a network and cannot be reproduced from the project. A
  `recipe` is a document in `recipes/`, synthesised locally, free and
  deterministic. An asset carries exactly the brief its kind takes:
  `AssetKind::is_prompted` vs `is_synthesized`, both under `is_generated`;
  never collapse them.
- **Compositing is ours; ffmpeg only decodes and encodes.** ffmpeg decodes
  sources to raw frames, our compositor produces each output frame (transforms,
  alpha, text), and raw frames are piped to ffmpeg to encode. Aspect,
  resolution, fps and bitrate are chosen per render.
- **Keyframes are generic**: `(property_path, [(t, value, easing)])` over any
  numeric property; the mechanism never knows which property it animates.
- **Core defines property types, never property values**: text colour is
  choosable; nothing writes "make text red".
- **New motion graphics are pages; the native graphics are frozen.** An html
  clip (**docs/pages.md**) does what shapes, icons and text animation do, and
  more. Native `text` and `color` stay as the fast path. `shape`, `icon`,
  arrows with `attach` / `follow`, text `reveal` / `number` and gradients get
  bug fixes only: a new graphics capability (a text shadow, a title preset)
  goes to pages and their motion kit. None is removed until pages capture on
  Windows and the tool-call record shows no new native shapes; removing a kind
  is a format change with a migration.
- **Audio is first-class**: audio tracks with keyframable volume, and music
  ducks under narration (`scorsese duck`, the `duck_music` tool).
- **The GUI is `egui`, in Rust, and deliberately thin**, a client of the same
  library logic as the CLI and MCP. It gets what a person reaches for with a
  mouse, often: scrub, select, nudge, trim, change a plain value. Anything with
  structure to it is a sentence to an assistant over MCP, never a menu. **Ship
  it simple, then iterate from use**: a first version's bar is *the user can
  start editing with it*.
- **The app follows the web app's visuals**: colours, type and spacing are
  decided on the web and the app follows; the web-only parts (assistant,
  accounts, library) are not mirrored.
- **MCP is a protocol, not a Claude feature.** Every client that speaks it gets
  the same tools; Claude is who we test against, not a dependency.
- **The web app's assistant runs on the model each project picks**
  (`crates/server/CLAUDE.md`): a product decision for our own client, while the
  tool surface still assumes nothing about who calls it.

### Crate map

`core` (model, format, validation) ← `compositor` (frames) ← `render` (ffmpeg
orchestration). `zimmer` (synthesis, no I/O) ← `providers` (every vendor call,
local synthesis, the brief-hash cache). `guide` (the agent docs, compiled in) ←
`cli` and `mcp` (thin wrappers over the same logic). `server` (HTTP, Postgres,
jobs) is a thin client of the same crates with **no editing logic of its own**:
what an endpoint needs that the CLI does not share belongs a layer down.
`golden` is test infrastructure; `web/` the React front-end; `app/` the egui
desktop app (its own workspace); `deploy/` the web app's stack; `tools/` what
the gates run.

**`zimmer` is ours**, and the owner's game engine **rusty** depends on it too,
so its public API is rusty's contract (`crates/zimmer/CLAUDE.md`).

## How we work

- **The project serves its development first.** First Claude looks for a more
  scalable way to improve the project faster, then improves the project itself
  (tools, features, infrastructure), and only then uses it. A video-editing
  session is the exception: its end result is the signal for all of that
  (`post-edit-feedback`).
  - **Infrastructure- then architecture-first.** A problem that bites or will
    bite more than once, a pattern worth adopting or a gold standard we lack is
    documented and fixed **before** continuing. It halts feature work, and gets
    its own issue when it carries its own responsibility.
  - **Agent velocity is first-class.** Agents drive this repo, often
    unattended: code that is readable by design and lean, and a fast CI.
- **Which model writes a branch.** Sonnet for what is purely front-end (the web
  app, or the desktop app's egui) and for mechanical work (removing a feature
  no longer used, a translation, moving code); Opus for the rest. A Sonnet's
  pull request carries the `minor` label.
- **The gates (push back before you build).** An idea becomes an issue only
  when all three hold; if any fails, push back instead of complying:
  1. **Understanding.** The user has a clear intent and Claude can restate it.
     If unsure, restate it and confirm; don't guess.
  2. **Value.** It adds real value. No busywork, no features for their own
     sake.
  3. **Craft.** It follows Rust good practice and the gold standards of
     video-editor architecture, the decided architecture included. If not, say
     so and propose the right shape.
- **Take the initiative.** A clear win without a downside, for scorsese, its
  videos or its development, is **implemented**, not proposed. **Changing a
  decision** (the decided architecture, the `project.json` format, the tool
  surface's shape, a rule in this file) is proposed and the user decides,
  unless one option is a plain win-win. **Making a description true or
  complete** (a doc, comment, help text, tool description, skill or
  `CLAUDE.md`) is never the exception. Nothing that spends the user's money (a
  provider generation, `make live-check`) is done on initiative: it is asked
  first, every time.
- **Flow:** idea → (issue) → branch → PR → CI green → merge; every change
  arrives as a pull request. A PR closes its issue by number; one leaving it
  open writes `refs #N`. The procedures are skills: batches, worktrees and
  cloud coders (`issue-batch`); gates, CI, the mutation signal and merging
  (`ci-merge`); a reviewer for a risky change (`independent-review`).
- **Gates vs. signals: block on correctness, inform on quality.** Signals
  (coverage, mutation, perf) never hold a merge. A branch that adds mechanism
  asks for a mutation run before it is readied.
- **A convention a machine can check is a gate, not a sentence.** Finding what
  can be enforced is Claude's job: a new convention ships with its check, and a
  written one a check could hold gets it on Claude's own say-so. A check has
  tests (a case it must flag, one it must pass), runs in `make gates` and CI,
  takes exceptions inline with a written reason, and says in its failure what
  is wrong and what to do. **What a check enforces is not repeated in prose**:
  the check speaks when something is wrong, which is when an agent needs to
  hear it.
- **Working unattended, decisions are written down, never waited on**: a
  judgement call an issue left open takes the default the issue, this file or
  Filmora 9 (for taste) points to, and the choice and its reason go on the
  issue or PR. A check only a human can do (a real window, real speakers,
  taste) never holds a merge. Questions during *planning* conversations are
  asked right away.
- **Memory is the developer's; what the project follows lives in the project.**
  Claude's local memory holds only the developer's preferences and their
  machine's setup. Anything the project should follow goes into this file, a
  skill, a doc or a tool description.
- **Agent docs state the rule, never its reasons or history**: the issue that
  made a rule keeps why.

## Repo-specific conventions

- **No real provider calls in tests, ever.** Every networked provider is mocked
  behind a trait, and golden renders use local fixture media only.
- **Infrastructure is code**: whatever a vendor lets Terraform manage is in
  `deploy/terraform`, never set by hand in a dashboard.
- **Secrets resolve in one order, through one resolver** (`providers`'
  `credentials` module, **docs/credentials.md**): the environment first, then
  the per-machine settings file. Never in `project.json`, code or fixtures.
- **Nested `CLAUDE.md` files** hold what matters only inside one folder; they
  load when a file there is read, so a rule needed earlier stays here.
- **Nothing in the codebase is temporary**, except small JSON or log files.

## Issues, labels & priority

- **Issues are how work is planned**, read cold by a future Claude, often
  unattended. Quote a vendor's figures from the vendor's own page, with the
  date; point at code by symbol and path, never by line number.
- **File what you notice: it is a duty, not an option.** A bug, a missing
  feature, a design gap or a quality-of-life fix is folded into the task when
  small or in the way, filed as an issue otherwise, never dropped. **A change
  that needs the user's approval is never folded in**: one that changes how the
  user sees or understands their existing data or project, changes stored data,
  or needs a migration. **A bug or a documentation fix never needs it**: filed
  without `planning` and merged without waiting, even when existing projects
  render differently (a format change still bumps the schema). An issue Claude
  files on its own carries the `agent` label.
  - **While the user is making a video with the Claude CLI, no issue is filed
    until the video is delivered**: the findings wait for `post-edit-feedback`.
- **Priority by label: infrastructure → architecture → bug → foundation →
  feature.** `infrastructure` is CI, harnesses and gates (a bug in the tooling
  ranks here); `architecture` is structure, conventions, crate boundaries and
  the `project.json` format; `foundation` is groundwork making the editor more
  complete; `feature` is a capability serving the videos. **documentation**
  never waits its turn. Priority orders what gets **merged**, never what gets
  **worked**.
- **Stage labels: at most one, and absence means ready.** `planning` (the
  design is not agreed yet) and `human` (needs a human end-to-end) both mean
  **do not start**, and they are the only thing that does.
- **Claude settles the design instead of asking for a stage label**: it states
  the decisions with a recommendation each, the user answers, and the answers
  go into the issue, filed startable. The exception Claude applies itself: a
  finding made **unattended** that needs the user's approval is filed with
  `planning` and raised at the next check-in, never a bug or a documentation
  fix.
- **The dependency graph is the plan.** Blocked by / Blocks and sub-issues;
  split by responsibility, never by parallelism. A `planning` issue that would
  change how another is built blocks that other one.

## Overrides

Any rule in this file may be overridden by the user's explicit say-so, in the
current prompt or a previous one. The **one exception**: an issue labelled
`planning` is never started while the label is on it; it says the work is not
ready, and the user may tell you to remove the label and then do it. The user
may also greenlight an issue blocked by another, which lifts that block.
