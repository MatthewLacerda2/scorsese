//! Tools that change something: bring media in, write the document, place and
//! trim clips, edit a brief, dissolve a cut, duck the music, set a clip's
//! volume, scale a run of clips, render.
//!
//! One file per tool. They were one file until the dissolve arrived and put
//! it over the size gate, which is the gate doing its job: tools that happen
//! to all be verbs are that many concerns, not one.

mod brief;
mod dissolve;
mod duck;
mod import;
mod pace;
mod place;
mod probe;
mod render;
mod trim;
mod volume;
mod write;

pub(crate) use brief::Rebrief;
pub(crate) use dissolve::Dissolve;
pub(crate) use duck::Duck;
pub(crate) use import::Import;
pub(crate) use pace::ScalePacing;
pub(crate) use place::PlaceClip;
pub(crate) use probe::Probe;
pub(crate) use render::Render;
pub(crate) use trim::TrimClip;
pub(crate) use volume::SetVolume;
pub(crate) use write::Write;

use scorsese_core::{Clip, Fps, Frames};
use serde_json::Value;

/// A required clip id argument, named so the refusal says which one was
/// missing.
fn clip_id<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{key}` is required: a clip id"))
}

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

/// A required id argument, and what it is an id of.
fn named<'a>(arguments: &'a Value, key: &str, what: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("`{key}` is required: {what}"))
}

/// A time argument in seconds, or `None` where it was left out.
///
/// A negative time is refused rather than clamped to zero, for the reason the
/// whole family of tools exists: the caller has said something that cannot be
/// true, and silently editing it into something that can be is how a clip ends
/// up somewhere nobody asked for.
fn seconds(arguments: &Value, key: &str) -> Result<Option<f64>, String> {
    let Some(value) = arguments.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let seconds = value
        .as_f64()
        .filter(|seconds| seconds.is_finite())
        .ok_or_else(|| format!("`{key}` has to be a number of seconds"))?;
    if seconds < 0.0 {
        return Err(format!(
            "`{key}` cannot be negative — the timeline starts at 0s"
        ));
    }
    Ok(Some(seconds))
}

/// Where a clip sits, said in both units at once.
///
/// Seconds because that is what was asked for, frames because that is what the
/// document now holds — and a caller that cannot read the frame back has no way
/// to tell which side of a rounding its cut landed on.
fn bounds(fps: Fps, clip: &Clip) -> String {
    let moment = |frames: Frames| format!("{:.2}s (frame {})", fps.seconds(frames), frames.get());
    let source = if clip.source_in == Frames::ZERO {
        format!("from the head of `{}`", clip.asset)
    } else {
        format!("opening {} into `{}`", moment(clip.source_in), clip.asset)
    };
    format!(
        "starts at {}, runs {:.2}s ({} frames) to {}, {source}",
        moment(clip.start),
        fps.seconds(clip.duration),
        clip.duration.get(),
        moment(clip.end())
    )
}
