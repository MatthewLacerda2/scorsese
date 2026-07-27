//! # scorsese-golden — the golden-render harness
//!
//! Responsibility: rendering tiny fixture projects and holding their output to
//! committed reference frames, as a **hard CI gate**.
//!
//! Every change from here on is a change to *pixels*, and pixels regress
//! silently — a transform half a pixel out, an easing curve flipped, a cut one
//! frame early. Nobody's eyes are in the loop for an agent-driven PR, so this
//! is what stands between "clippy is green" and "the video is actually right".
//! It exists before the compositor grows features so that every capability
//! lands with a golden test from the day it is written.
//!
//! How a fixture works:
//!
//! 1. A directory under `fixtures/` holds a real `project.json`, a
//!    `fixture.json` saying how to conjure its media and how to render it, and
//!    an `expected/` directory of reference PNGs.
//! 2. [`harness::run`] generates the media with ffmpeg's synthetic sources into
//!    a scratch project directory, renders it, and pulls out the frames the
//!    fixture nominates — with `scorsese_render::frames`, which is a shipped
//!    capability this crate calls rather than one it owns.
//! 3. Each frame is compared to its reference by [`compare()`] — per-channel SSIM
//!    and mean absolute error, both with tolerance.
//!
//! **Comparison is perceptual, never byte-equality of encoded output.** Two
//! ffmpeg builds encoding the same frames produce different bitstreams, so the
//! bytes of an `.mp4` are not ours to assert on. Frames are.
//!
//! Re-blessing is a deliberate act: `UPDATE_GOLDENS=1 cargo test -p
//! scorsese-golden` rewrites the references, and because they are PNGs the
//! change arrives in review as an image diff a human can actually judge. When
//! that is legitimate is written down in `docs/golden-renders.md`.
//!
//! Boundary: test infrastructure. Nothing ships it, nothing depends on it, and
//! no product behaviour may be implemented here — a capability that turns out
//! to be generally useful belongs in `scorsese-render` and gets used from here.

pub mod compare;
pub mod fixture;
pub mod harness;

pub use compare::{Difference, Tolerance, compare};
pub use fixture::{Fixture, FixtureError, Manifest, Recipe};
pub use harness::{
    GoldenError, Mismatch, Mismatches, Mode, Outcome, UPDATE_VARIABLE, assert_matches, run,
};
