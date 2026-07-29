//! Dissolving one shot into the next.
//!
//! A dissolve is nothing but one clip's opacity falling while another's rises.
//! Scorsese could always produce one; what it could not do was *say* one, and
//! the difference was pure bookkeeping — an extra video track, and two opacity
//! tracks of four keyframes each, hand-written and reasoned about.
//!
//! This is [`crate::fade_in`] and [`crate::fade_out`] in their two-clip case,
//! and it keeps their bargain exactly: what it writes is **ordinary opacity
//! keyframes**, which stay visible, editable and deletable afterwards. Nothing
//! new reaches the renderer, nothing new enters the format, and a dissolve
//! composes with a move or a turn for free because none of them know about
//! each other.
//!
//! ## Why it lives here and not in `scorsese-core`
//!
//! Because it names `opacity`. Core defines property *types* and never
//! property *values*, which is why [`scorsese_core::dip`]'s ducking takes the
//! property to write as an argument and lives there, while the fades name
//! their property and live here. A dissolve is a fade's sibling, so it is
//! here.
//!
//! ## The overlap decision
//!
//! Clips on one track may touch but never overlap, and that rule is
//! load-bearing: with integer frames a cut is a fact rather than a tolerance.
//! So two clips that merely meet have nowhere on their own track for a
//! dissolve to happen, and something has to give.
//!
//! **This moves the incoming clip onto a track of its own and overlaps it
//! there.** The alternative — refusing unless the caller has already arranged
//! two tracks — is smaller, and it was rejected deliberately: the bookkeeping
//! this exists to remove was *both* halves, the track juggling as much as the
//! ramps, and an operation that fixes only the easier half leaves the caller
//! doing the part they were most likely to get wrong. It is also what a person
//! means when they drag one clip onto another.
//!
//! Three things keep that from being a surprise. It **refuses** when the clips
//! do not meet, because a dissolve with no crossover has no defined shape. The
//! tracks it writes are **signed** [`TOOL`], so running it again replaces its
//! own work and leaves a hand-authored fade alone — the guarantee ducking
//! already gives. And the incoming clip goes **above** the outgoing one, which
//! is what makes it the thing that arrives.

mod place;

use scorsese_core::{Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, Project, PropertyPath};

use crate::properties::path;

pub use place::{DissolveError, Placed};

/// Who signs the keyframe tracks this module produces.
///
/// A constant rather than a caller's choice, for the reason ducking gives: the
/// whole value of a signature is that the *next* run recognises it, and a name
/// that varies is a name that never matches.
pub const TOOL: &str = "dissolve";

/// Dissolves `from` into `to` over `duration` frames.
///
/// The outgoing clip fades out over its last `duration` frames; the incoming
/// one is pulled back to start as that fade begins, moved to a track of its
/// own, and fades in over the same span. What lands on disk is four ordinary
/// keyframes across two tracks, and nothing else.
///
/// # Errors
///
/// If either clip is missing, they are not both on video tracks, they do not
/// meet, or the dissolve is longer than one of them can give. See
/// [`DissolveError`], which says which and names the clip.
pub fn dissolve(
    project: &mut Project,
    from: &ClipId,
    to: &ClipId,
    duration: Frames,
) -> Result<Placed, DissolveError> {
    let placed = place::overlap(project, from, to, duration)?;
    ramp(project, from, Ramp::Out, duration)?;
    ramp(project, to, Ramp::In, duration)?;
    Ok(placed)
}

/// Which way a clip's opacity goes.
#[derive(Debug, Clone, Copy)]
enum Ramp {
    /// From nothing to solid, over the clip's first `duration` frames.
    In,
    /// From solid to nothing, over its last `duration` frames.
    Out,
}

/// Writes one side of the crossover onto one clip.
fn ramp(
    project: &mut Project,
    id: &ClipId,
    direction: Ramp,
    duration: Frames,
) -> Result<(), DissolveError> {
    let clip = clip_mut(project, id).ok_or_else(|| DissolveError::NoSuchClip {
        clip: id.to_string(),
    })?;
    let total = clip.duration;
    let keyframes = match direction {
        Ramp::In => vec![at(Frames::ZERO, 0.0), at(duration, 1.0)],
        // The ramp reaches zero at the clip's end — the frame after its last —
        // so the picture is still just barely there on the final frame and goes
        // out exactly on the cut. The same shape `fade_out` writes.
        Ramp::Out => vec![
            at(Frames(total.get().saturating_sub(duration.get())), 1.0),
            at(total, 0.0),
        ],
    };
    let track = KeyframeTrack::new(PropertyPath::new(path::OPACITY), keyframes).generated_by(TOOL);
    // Replaces only this tool's own tracks, so a second run leaves one
    // dissolve and a hand-written fade beside it survives untouched.
    clip.replace_generated(TOOL, vec![track]);
    Ok(())
}

/// One keyframe, linear — a dissolve is the neutral case and a curve is the
/// author's choice, made by editing the easing this writes.
fn at(t: Frames, value: f64) -> Keyframe {
    Keyframe {
        t,
        value,
        easing: Easing::Linear,
    }
}

/// The clip with this id, mutably, wherever it is.
fn clip_mut<'a>(project: &'a mut Project, id: &ClipId) -> Option<&'a mut Clip> {
    project
        .tracks
        .iter_mut()
        .flat_map(|track| track.clips.iter_mut())
        .find(|clip| &clip.id == id)
}
