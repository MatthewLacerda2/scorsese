# app/ — the desktop window

The one part of scorsese a person looks at. An `egui` application in Rust,
shipping as a standalone binary on Windows, macOS and Linux.

```
cargo run --manifest-path app/Cargo.toml
cargo run --manifest-path app/Cargo.toml -- path/to/teaser.scor   # skip the dialog
```

The path argument is a convenience for development, not a documented flag — the
shipped way in is the Open dialog.

**On Linux, building needs the ALSA development headers.** Sound goes through
`cpal`, which reaches `alsa-sys`, whose build script asks pkg-config for
`alsa.pc` and fails the compile without it:

```
sudo apt-get install libasound2-dev   # Debian, Ubuntu
sudo pacman -S alsa-lib               # Arch
sudo dnf install alsa-lib-devel       # Fedora
```

A sound *card* is not required. Without one the preview plays the picture
silently and says why, which is what CI does. `make app` checks for the headers
and names the package rather than letting a build script panic three crates
down.

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

`make gates` runs them too, and only when the branch touches `app/` — the
whole point of the separate workspace is that a headless change never pays for
this dependency tree, and a gate runner that charged every commit for it would
give that back. So a change under `app/` gets these four locally before the
push, and a change that leaves the app alone is told they were skipped rather
than shown a green that covered nothing.

They run here and nowhere else. `app/` is its own workspace so that a graphics
dependency tree never slows a headless change, which also means
`cargo test --workspace` cannot reach them — so this job is the only place they
ever execute.

None of them open a window. What they cover is the logic behind the drawing:
the frame-to-pixel transform, lane ordering, snapping, trim arithmetic, and
what a refused edit leaves on disk. That is where the bugs are; the drawing is
checked by looking at it.

## Looking at it without a display

`app/tests/panels/` renders the whole window offscreen through `egui_kittest`
— no window, no display, no X server — and holds each panel to a reference
image in `app/tests/snapshots/`.

That exists because six panels were built before it did and **not one frame of
any of them had ever been looked at.** Every other test here covers the logic
*behind* the drawing, because the drawing was unreachable.

```
cargo test --manifest-path app/Cargo.toml --test panels
UPDATE_SNAPSHOTS=1 cargo test --manifest-path app/Cargo.toml --test panels
```

Comparison is **with tolerance**, never byte-for-byte: a GPU rasterising text
is no more deterministic across drivers than an encoder is across versions,
which is the same reason `docs/golden-renders.md` gives for renders.

**The rule that matters carries over unchanged: re-blessing a reference to make
a test pass is never legitimate.** A snapshot changes when the interface was
meant to change, and the new picture is *looked at* before it is committed.
`UPDATE_SNAPSHOTS=1` writes them; it does not decide they are right.

A fixture project is named for its label alone — no process id, no counter —
because the window puts the directory's name in its menu bar, and a name that
changed between runs would change the picture between runs. A snapshot that
cannot reproduce itself is not a reference.

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
lands a frame early is what a step button is for. Playback drops frames rather
than slowing down, so what it shows about pacing is true even when compositing
cannot keep up.

**It plays the sound too, and the sound is the clock.** A dropped video frame
is invisible; a dropped sample is a click and a stretched one is a pitch
change, so audio cannot be made to follow anything — the mix's position *is*
the playhead, and the picture is whichever frame we managed to draw for it. The
wall clock is still underneath, for the moment before the mix is ready, for a
film with nothing audible in it, and for a machine with no sound card. Every
sample comes from the renderer's own mixer, so what you hear is what a render
delivers.

Scrubbing and stepping are silent on purpose. A frame step is a thirtieth of a
second — a click, not a note — and scrubbing audio needs a scheme of its own
rather than whatever fell out of playback.

The furniture arrives issue by issue — the inspector next. See #13.
