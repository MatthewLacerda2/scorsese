//! Making audio fixtures, and asking a rendered file what it sounds like.
//!
//! Everything here talks in **loudness over a window of time**, never in
//! individual samples. A rendered soundtrack has been resampled and through a
//! lossy encoder by the time it comes back, so a single sample says nothing;
//! "is there sound here, and roughly how much" is the question the pipeline can
//! actually be held to.

pub mod fixtures;
pub mod listen;

pub use fixtures::{RATE, SHOT_LEVEL, talking_picture, tone_asset, with_volume};
pub use listen::{assert_audible, assert_silent, has_soundtrack, level, soundtrack};
