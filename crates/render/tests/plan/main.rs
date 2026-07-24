//! Sequencing a timeline into a render, with no ffmpeg anywhere in sight.
//!
//! Everything about *what is on screen when* is decided here, so this is where
//! it gets pinned down exhaustively — no encoding, no temp files, no external
//! binary.

#[path = "../common/mod.rs"]
mod common;

mod range;
mod refusals;
mod sequencing;
