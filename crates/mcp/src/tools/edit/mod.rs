//! Tools that change something: bring media in, write the document, make or
//! retime an image sequence, place, trim, move, remove, group and ungroup
//! clips, set a clip's plain values, animate one of them, send one along an
//! arrow, edit a brief, dissolve a cut, duck the music, set a clip's volume,
//! scale a run of clips, render.
//!
//! One file per tool. They were one file until the dissolve arrived and put
//! it over the size gate, which is the gate doing its job: tools that happen
//! to all be verbs are that many concerns, not one.

mod animate;
mod brief;
mod clip;
mod dissolve;
mod duck;
mod follow;
mod group;
mod import;
mod pace;
mod place;
mod probe;
mod relocate;
mod remove;
mod render;
mod sequence;
mod sequenced;
mod trim;
mod volume;
mod write;

pub(crate) use animate::ClipAnimate;
pub(crate) use brief::Rebrief;
pub(crate) use clip::ClipSet;
pub(crate) use dissolve::Dissolve;
pub(crate) use duck::Duck;
pub(crate) use follow::ClipFollow;
pub(crate) use group::{ClipGroup, ClipUngroup};
pub(crate) use import::Import;
pub(crate) use pace::ScalePacing;
pub(crate) use place::PlaceClip;
pub(crate) use probe::Probe;
pub(crate) use relocate::ClipMove;
pub(crate) use remove::ClipRemove;
pub(crate) use render::Render;
pub(crate) use sequence::Sequence;
pub(crate) use trim::TrimClip;
pub(crate) use volume::SetVolume;
pub(crate) use write::Write;

use std::collections::BTreeSet;

use scorsese_core::{Clip, ClipId, Fps, Frames};

/// Seconds on the project's grid, at least one frame when anything was asked
/// for — a ramp rounded to no frames is a switch, not a ramp.
fn frames(seconds: f64, fps: f64) -> Frames {
    if seconds <= 0.0 {
        return Frames::ZERO;
    }
    Frames(((seconds * fps).round() as u64).max(1))
}

/// A time argument in seconds, refused when it is negative.
///
/// Refused rather than clamped to zero, for the reason the whole family of
/// tools exists: the caller has said something that cannot be true, and
/// silently editing it into something that can be is how a clip ends up
/// somewhere nobody asked for. Whether it is a number at all is the shared
/// argument path's to say; this is the check JSON's types cannot make.
fn seconds(given: Option<f64>, key: &str) -> Result<Option<f64>, String> {
    match given {
        Some(seconds) if seconds < 0.0 => Err(format!(
            "`{key}` cannot be negative — the timeline starts at 0s"
        )),
        given => Ok(given),
    }
}

/// The clips a call names, as a set — a repeated id is one clip, not two —
/// refused when it names none, since an edit of no clips is a call that
/// meant something else.
fn clip_set(ids: &[String]) -> Result<BTreeSet<ClipId>, String> {
    let ids: BTreeSet<ClipId> = ids.iter().map(|id| ClipId::new(id.as_str())).collect();
    if ids.is_empty() {
        return Err("`clips` must name at least one clip".to_owned());
    }
    Ok(ids)
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
