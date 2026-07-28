# app/ — the desktop window

The one part of scorsese a person looks at. An `egui` application in Rust,
shipping as a standalone binary on Windows, macOS and Linux.

```
cargo run --manifest-path app/Cargo.toml
cargo run --manifest-path app/Cargo.toml -- path/to/teaser.scor   # skip the dialog
```

The path argument is a convenience for development, not a documented flag — the
shipped way in is the Open dialog.

## What it is for

Scrub, select, nudge, trim, change a plain value. The operations a person
reaches for **with a mouse, often**.

Anything with structure to it — building a cut, scoring it, fitting music,
generating shots — is a sentence to an assistant over MCP, not a menu here. A
window rich enough to do the editing would be a second and weaker way to do
everything, and this one stays thin so it can keep being reshaped from use.

## Its own workspace, on purpose

`app/` is **not** a member of the root cargo workspace. egui reaches a window
through a graphics stack — winit, wgpu, the platform's own toolkit — and none of
that has any business being compiled by `cargo test --workspace`, which every
headless change runs.

The cost is that it inherits nothing, the lint policy included. So the policy is
restated in `app/Cargo.toml` and `app/clippy.toml`, and
`tools/lint/tests/policy.rs` fails if any separate root drifts from the others.

## Its tests are a gate

`make app` runs this crate's format, clippy, build **and tests**, and the
`desktop app` CI job runs the same four. A failing test here blocks a merge
exactly as a failing workspace test does.

They run here and nowhere else. `app/` is its own workspace so that a graphics
dependency tree never slows a headless change, which also means
`cargo test --workspace` cannot reach them — so this job is the only place they
ever execute.

None of them open a window. What they cover is the logic behind the drawing:
the frame-to-pixel transform, lane ordering, snapping, trim arithmetic, and
what a refused edit leaves on disk. That is where the bugs are; the drawing is
checked by looking at it.

## Invariants

- **The GUI is a client**, not a layer underneath. It reads and writes the
  project through `scorsese-core`, so the document on disk is the only model and
  the window can never disagree with what the CLI and the MCP server see.
- **`core` and `cli` never touch a display.** That does not bend for this.
- **Previews come from `scorsese-compositor`** at reduced resolution — the same
  compositor a render uses. There is never a second rendering path, because a
  preview that draws the picture its own way is a preview that lies.
- **ffmpeg goes through `scorsese-render`'s command builder**, bundled beside
  the binary in shipped builds.

## What is here now

The window and the four-panel frame: preview, timeline, inspector, project
files. It opens a project, says what is in it, and lists every validation
problem when it will not load.

The preview shows the picture at the playhead, composited at a reduced raster
by `scorsese-render`'s own still — the render pipeline with the encoder taken
out — with a transport under it: jump to start, back a frame, play or pause,
forward a frame, jump to end. A step is **one frame**, because whether a cut
lands a frame early is what a step button is for. Playback runs on the wall
clock and drops frames rather than slowing down, so what it shows about pacing
is true even when compositing cannot keep up. **No sound yet**: playing audio
in step with the picture is a second clock to keep in sync, and it gets its own
issue once scrubbing feels right.

The furniture arrives issue by issue — the inspector next. See #13.
