//! The numbers under a soundtrack: what one sample of the mix is made of.
//!
//! No ffmpeg, no files, no timeline. The end-to-end audio tests next door ask
//! *"is there sound here, and is it getting louder?"* — the right question of a
//! render, and one that several different arithmetics answer the same way. A
//! window measurement cannot tell summing from subtracting, or an interpolated
//! ramp from a stepped one.
//!
//! So these assert the value instead: this sample is *those two sources added*,
//! this multiplier is *a quarter of the way from 0.0 to 1.0*. Every number here
//! is exactly representable in `f32`, which is why they are compared for
//! equality rather than within a tolerance.

mod gain;
mod mix;

use scorsese_core::{Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};
use scorsese_render::audio::path;

/// A `volume` track through `(frame, multiplier)` points, travelling linearly.
pub fn volume(points: &[(u64, f64)]) -> KeyframeTrack {
    keyframed(path::VOLUME, points)
}

/// The same, under any property path — so a mixer that reads the wrong one has
/// something to be caught by.
pub fn keyframed(property: &str, points: &[(u64, f64)]) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        points
            .iter()
            .map(|&(t, value)| Keyframe {
                t: Frames(t),
                value,
                easing: Easing::Linear,
            })
            .collect(),
    )
}
