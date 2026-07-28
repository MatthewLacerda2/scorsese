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

The furniture arrives issue by issue — the preview, the timeline drawing, the
inspector, the files list. See #13.
