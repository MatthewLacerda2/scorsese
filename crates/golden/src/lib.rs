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
//! 2. [`run()`] generates the media with ffmpeg's synthetic sources into
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
//! That reasoning covers the encoder at the end of a render. ffmpeg is also
//! the decoder at the start of one, and the tolerances were never sized to
//! absorb a different one — so each fixture records the ffmpeg its references
//! were blessed under, and a failure says which one produced the frames it is
//! complaining about. It is provenance for a failure to quote, never a check
//! of its own: [`Decoder`].
//!
//! It is also why the fixture suite runs on Linux and skips itself anywhere
//! else. References are blessed there and CI compares them there; off it, a
//! comparison measures the local decode path as much as it measures the
//! compositor. That decision lives in `tests/goldens.rs` rather than in this
//! API — nothing here inspects the target — and `docs/golden-renders.md`
//! argues it, including why it is keyed on the platform and not on
//! [`Decoder`].
//!
//! Re-blessing is a deliberate act: `UPDATE_GOLDENS=1 cargo test -p
//! scorsese-golden` rewrites the references, and because they are PNGs the
//! change arrives in review as an image diff a human can actually judge. When
//! that is legitimate is written down in `docs/golden-renders.md`.
//!
//! Boundary: test infrastructure. Nothing ships it, nothing depends on it, and
//! no product behaviour may be implemented here — a capability that turns out
//! to be generally useful belongs in `scorsese-render` and gets used from here.

mod compare;
mod fixture;
mod harness;

pub use compare::{Difference, SizeMismatch, Tolerance, compare};
pub use fixture::{Fixture, FixtureError, Manifest, Recipe, RenderSpec};
pub use harness::{
    Decoder, GoldenError, Mismatch, Mismatches, Mode, Outcome, RECORD_FILE, SetupError,
    UPDATE_VARIABLE, assert_matches, run,
};
