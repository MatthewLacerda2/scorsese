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
path. The format is documented in
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
```

Requires Rust (stable) and, for now, `ffmpeg`/`ffprobe` on your PATH —
shipped builds will bundle ffmpeg as a sidecar later. ffmpeg only decodes and
encodes; all compositing (transforms, alpha, text) is done by our compositor.

Set `SCORSESE_FFMPEG` / `SCORSESE_FFPROBE` to point at specific binaries
instead of whatever is on PATH.

## Using it so far

```sh
scorsese new teaser.scor --name "Product teaser"
scorsese import ~/footage/skyline.mp4 --project teaser.scor
scorsese import ~/music/bed.wav --project teaser.scor

scorsese assets --project teaser.scor            # what's in the pool
scorsese assets --verify --project teaser.scor   # re-hash to catch changed files
scorsese assets gc --project teaser.scor         # what nothing references
scorsese assets gc --delete --project teaser.scor
```

Importing **copies** the file into `assets/`, hashes it, and asks ffprobe what
it is. Import the same content twice and you get the same asset back — assets
are entities, and two clips pointing at one asset is the intended shape.

Rendering, generation, and the rest of the timeline commands are still to
come; the [issue tracker](https://github.com/MatthewLacerda2/scorsese/issues)
is the plan.

## Crate map

| Crate | Responsibility |
| --- | --- |
| `crates/core` | Timeline model, assets table, keyframes, serde `project.json` format, validation |
| `crates/compositor` | Frame rendering — CPU (tiny-skia) first, wgpu later behind the same trait |
| `crates/render` | ffmpeg orchestration: probe, decode pipes, encode pipe, render settings |
| `crates/providers` | Veo + ElevenLabs clients, prompt-hash cache, generation states |
| `crates/cli` | The headless `scorsese` binary: `new`, `import`, `render`, `generate`, `assets`, `diff` |
| `crates/mcp` | MCP server — the same operations as MCP tools for Claude agents |
| `app/` | Tauri GUI (placeholder — arrives with its seed issue) |

`core` and `cli` never touch a display. Each crate's `lib.rs` states its
responsibility and what it must never depend on.

## Contributing

The workflow — gates, labels, issue conventions, merge discipline — lives in
[CLAUDE.md](CLAUDE.md). Work flows idea → issue → branch → PR → CI green →
merge; the issue dependency graph is the plan.
