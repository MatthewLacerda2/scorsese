//! Tools that change something: write the document, dissolve a cut, duck
//! the music, scale a run of clips, render.
//!
//! One file per tool. They were one file until the dissolve arrived and put
//! it over the size gate, which is the gate doing its job: four tools that
//! happen to all be verbs are four concerns, not one.

mod dissolve;
mod duck;
mod pace;
mod probe;
mod render;
mod write;

pub(crate) use dissolve::Dissolve;
pub(crate) use duck::Duck;
pub(crate) use pace::ScalePacing;
pub(crate) use probe::Probe;
pub(crate) use render::Render;
pub(crate) use write::Write;

use scorsese_core::Frames;
use serde_json::Value;

/// A number argument, or its default.
fn number(arguments: &Value, key: &str, fallback: f64) -> f64 {
    arguments
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
}

/// Seconds on the project's grid, at least one frame when anything was asked
/// for — a ramp rounded to no frames is a switch, not a ramp.
fn frames(seconds: f64, fps: f64) -> Frames {
    if seconds <= 0.0 {
        return Frames::ZERO;
    }
    Frames(((seconds * fps).round() as u64).max(1))
}
