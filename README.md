# scorsese

A cross-platform video editor built for agentic workflows. A human — or a
Claude agent — assembles a video from a JSON project file and renders it
headlessly; a GUI exists for humans to scrub, tweak, and review. Named as a
nod to Martin Scorsese.

## The project-directory model

A project is a directory:

```
myvideo.scor/
  project.json    # the whole edit: assets table, tracks, clips, keyframes
  assets/         # imported media, copied in on import
  generated/      # provider outputs, content-addressed by prompt hash
  cache/          # rebuildable scratch (gitignored)
```

Every path in `project.json` is relative to the project root — a project
survives `scp -r` to another machine. Assets are entities (id, kind, path,
sha256, probed metadata); clips on tracks reference assets by id, never by
path. The edit is authored against a **timeline framerate** — an exact
rational, so 29.97 is 30000/1001 and not a rounded float — and every clip and
keyframe time is a whole frame count on that grid; the framerate you *render*
at is a separate, per-render choice. The format is documented in
[docs/project-format.md](docs/project-format.md) and is meant to be
hand-written.

## Sketch → GO

Clips backed by a generative prompt (Veo video, ElevenLabs TTS) start as
**sketches**: they render as slug cards — the prompt text on a gray card — so
you can preview the entire cut for $0. When the cut is right, **GO**
(`scorsese generate`) generates only sketch and stale assets, reporting the
cost estimate before spending. Generated media is cached by prompt hash; an
unchanged prompt is never paid for twice. Editing a prompt after generation
marks the asset **stale**, back to a slug card until the next GO.

## Building

```sh
cargo build          # builds the workspace, including the `scorsese` CLI
cargo test           # runs every crate's tests

make setup           # once per clone: installs the committed git hooks
make gates           # everything CI blocks on — run this before opening a PR
make help            # every target, and which of them are gates
```

`make setup` points git at `.githooks/`, so `make pre-commit` — formatting and
the size gate, no build, well under a second — runs before each commit and a
file over the line limit never reaches a branch. It is repository config, so
one run covers every worktree. A deliberate work-in-progress commit gets
through with `git commit --no-verify`; everything the hook skips is in
`make gates`.

The **Makefile is the list of gates**: `make help` prints format, size,
clippy, docs, test and deny, each running exactly the command CI runs. Every
individual target is runnable on its own — `make size`, `make clippy` — and
`make format-fix` rewrites what the format gate objects to. `make deny` and
`make coverage` need tools that are not part of the toolchain pin; both say
how to install themselves rather than failing obscurely.

Signals are deliberately not in `make gates`:

```sh
make coverage        # which pub items no test reaches (needs cargo-llvm-cov)
```

Coverage is a **signal, not a gate**: there is no threshold and it never fails
a build. It exists to answer one question nothing else here answers — which
`pub` items no test reaches at all, which `clippy`'s `dead_code` cannot see
because nearly everything in these crates is `pub`. What it does *not* say is
whether a test asserts anything; an executed line is not a checked one, and
break-testing is what settles that. The `coverage` job in
`.github/workflows/ci.yml` documents what is excluded and why.

Requires `make` for the targets above — every recipe is one plain `cargo`
invocation, so they can be read off and run by hand where it is missing.
Requires `rustup` — `rust-toolchain.toml` pins the exact compiler, so the
right one installs itself on first build and your local `clippy` matches CI's
lint for lint. Also, for now, `ffmpeg`/`ffprobe` on your PATH — shipped builds
will bundle ffmpeg as a sidecar later. ffmpeg only decodes and encodes; all
compositing (transforms, alpha, text) is done by our compositor.

Set `SCORSESE_FFMPEG` / `SCORSESE_FFPROBE` to point at specific binaries
instead of whatever is on PATH.

## Using it so far

```sh
scorsese new teaser.scor --name "Product teaser"          # 30fps timeline
scorsese new broadcast.scor --fps 30000/1001              # 29.97, exactly
scorsese import ~/footage/skyline.mp4 --project teaser.scor
scorsese import ~/music/bed.wav --project teaser.scor

scorsese assets --project teaser.scor            # what's in the pool
scorsese assets --verify --project teaser.scor   # re-hash to catch changed files
scorsese assets gc --project teaser.scor         # what nothing references
scorsese assets gc --delete --project teaser.scor

scorsese check --project teaser.scor             # problems and warnings, no render
scorsese check --verify --project teaser.scor    # ...and re-hash the media too

scorsese render --project teaser.scor --out teaser.mp4
scorsese render --project teaser.scor --out cut.mp4 --range 90:210
scorsese render --project teaser.scor --out small.mp4 \
    --resolution 1280x720 --fps 60 --bitrate 6M
scorsese render --project teaser.scor --out teaser.mp4 \
    --sample-rate 48000 --audio-bitrate 192k
scorsese render --project teaser.scor --out legacy.avi
scorsese render --project teaser.scor --out legacy.avi --video-codec h264
```

The timeline framerate is chosen once, at `new`, and defaults to 30. It is the
grid the edit is authored against, not the rate you export at.

Importing **copies** the file into `assets/`, hashes it, and asks ffprobe what
it is. Import the same content twice and you get the same asset back — assets
are entities, and two clips pointing at one asset is the intended shape.

Putting clips on tracks means editing `project.json` for now — the format is
documented and meant to be written by hand or by an agent.

`check` answers one question — *will this render?* — and answers it about the
document **and** the media it points at. A file a clip needs and cannot find is
a problem and fails; a file that changed since it was imported, or was never
probed, is a warning and does not. An asset nothing references is never worse
than a warning, whatever state it is in, because no render can trip over it —
`assets gc` is the answer to that one. Sketch clips awaiting generation are the
normal state of a project before GO and are never reported at all. Existence is
always checked; `--verify` adds re-hashing, which costs a read of every file in
the pool and only ever produces warnings, so it is asked for rather than
assumed.

`render` walks the timeline and writes a file. Resolution, framerate, and
bitrate are chosen per render, never stored in the project; a render at a rate
other than `timeline_fps` conforms from the grid. Sources of another shape are
letterboxed rather than stretched, and holes in the timeline render black.
`--range` renders a slice of the timeline in frames, which is the cheap way to
check one cut without re-encoding everything.

The shape of the delivered file — the container and the codecs in it — is a
render setting like the rest. `--out`'s extension supplies the default, so
naming the file is usually the whole decision, and each container gets the
codecs it is expected to carry rather than H.264 in everything. A combination
scorsese does not write is refused before anything is encoded, because a
plausible-looking wrong file costs more than a refusal.
**docs/output-formats.md** is the accepted set and why it is that short.

Clips can be **animated**: opacity, position, and scale are keyframed
properties, evaluated per frame and drawn by our own compositor rather than by
ffmpeg. `docs/project-format.md` lists the property paths and what they mean,
and `scorsese check` warns — never fails — when a keyframe track names one
nothing animates, suggesting what a typo was probably meant to be.
Scale is centre-anchored, and a fade is nothing but opacity keyframes — so it
composes with a move or a zoom for free.

**Video tracks layer.** They composite bottom to top in the order they appear
in the project, each with its own transforms and opacity, so an overlay is just
a clip on a higher track. A clip chooses how it meets the raster with `fit`:
scaled to **fit** inside it (the default, leaving transparent around it so a
narrower clip shows the one beneath at the sides), scaled to **fill** it with
the overflow cropped, or left at its own **native** size resting centred — which
is how a logo is placed at the size it was drawn. A hole in a track lets the
tracks below through; only a stretch with nothing on any track is black.

**Renders have sound.** Audio tracks are placed, trimmed, and mixed together
the same way video tracks are stacked — several playing at once are summed,
and `volume` is an ordinary keyframed property, so a fade-out and a mute are
the same mechanism at different values. A **video clip's own audio is mixed
too**: the dialogue on an interview is heard because the clip is on the
timeline, and muting it under a voiceover is `volume: 0.0` on that clip.
Picture decides how long a render is: a music bed running past the last shot is
cut there and said so in the report.
A project with no audio produces a file with no audio stream, which is not the
same as a stream of silence.

**Renders draw text.** A `text` asset carries its string inline and a `style`
saying how it looks — font, size, colour, alignment, where it wraps — and the
compositor draws it, wrapping what is too wide and ending with an ellipsis
what is too tall. It composites like any other layer, so a title fades with
`opacity` and slides with `transform.position.*`, the same properties a video
clip uses.

Two faces ship with scorsese, committed to the repository under
`crates/compositor/fonts/` with their licence beside them:

| `font` | face | stands in for | licence |
| --- | --- | --- | --- |
| `sans` (default) | Liberation Sans | Arial | SIL OFL 1.1 |
| `serif` | Liberation Serif | Times New Roman | SIL OFL 1.1 |

They are shipped rather than looked up on the system because a system font
resolves to a different file on every platform, and text has to render
identically everywhere for the golden-render gate to mean anything. They are
defaults, not the whole vocabulary: `font` may instead name a font file the
project carries, like `assets/Inter-Regular.ttf`. Glyph outlines are read with
[skrifa](https://crates.io/crates/skrifa) and filled by the same tiny-skia
rasteriser every other layer goes through.

A clip awaiting generation still refuses to render rather than standing in as
a slug card. Slug cards and generation each have an issue; the
[issue tracker](https://github.com/MatthewLacerda2/scorsese/issues) is the
plan.

## Crate map

| Crate | Responsibility |
| --- | --- |
| `crates/core` | Timeline model, assets table, keyframes, serde `project.json` format, validation |
| `crates/compositor` | Frame rendering — CPU (tiny-skia) first, wgpu later behind the same trait; text, and the two fonts scorsese ships |
| `crates/render` | ffmpeg orchestration: probe, decode pipes, the audio mix, encode pipe, render settings |
| `crates/providers` | Veo + ElevenLabs clients, prompt-hash cache, generation states |
| `crates/cli` | The headless `scorsese` binary: `new`, `import`, `render`, `generate`, `assets`, `diff` |
| `crates/mcp` | MCP server — the same operations as MCP tools for Claude agents |
| `crates/golden` | Test infrastructure: the golden-render gate ([docs](docs/golden-renders.md)) |
| `app/` | Tauri GUI (placeholder — arrives with its seed issue) |

`core` and `cli` never touch a display. Each crate's `lib.rs` states its
responsibility and what it must never depend on.

## Contributing

The workflow — gates, labels, issue conventions, merge discipline — lives in
[CLAUDE.md](CLAUDE.md). Work flows idea → issue → branch → PR → CI green →
merge; the issue dependency graph is the plan.
