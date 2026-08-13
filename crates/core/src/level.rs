//! Holding one clip's property at a value, or bringing it to one.
//!
//! This is the mechanism behind **setting a clip's volume** — a level, or a
//! mute, or a fade — but like [`crate::dip`] it is written without ever naming
//! `volume`, because the meaning of that path belongs to the mixer that
//! implements it. A caller hands in the [`PropertyPath`] and the value; this
//! module decides which keyframes say so.
//!
//! ```text
//!   flat            ─────────────────────────   one keyframe, held
//!
//!   ramp    ────────╮                           two keyframes, linear
//!                    ╲
//!                     ╰────────────────────     `at` … `at + over`
//! ```
//!
//! What comes out is an ordinary [`KeyframeTrack`] — the same points a hand
//! would place, evaluated by the same [`KeyframeTrack::value_at`] — and it is
//! **unsigned**: nothing here signs its work the way ducking does, because
//! there is nothing to recognise later. A level is a decision about the clip,
//! not a pattern to be recomputed when its neighbours move.
//!
//! ## One property, one lane
//!
//! Setting a level **takes the property over**: every track already animating
//! it on that clip is replaced, and [`Levelled::replaced`] hands them back so a
//! caller can say what went. That is deliberate, and the alternative is worse —
//! two tracks on one property is a document where the second one silently never
//! plays, since each renderer resolves a path by finding *the* track for it.
//!
//! So a level set over a ducked clip removes the dip, and the caller is
//! expected to say so rather than let it be discovered on playback.

use crate::keyframe::{Easing, Keyframe, KeyframeTrack, PropertyPath};
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId};
use crate::validate::ValidationErrors;

/// What a property is to read over a clip, and how it gets there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    /// One value, for the whole clip. A single keyframe at the clip's first
    /// frame, which holds from there — the plainest thing the format can say.
    Flat(f64),
    /// A travel between two values, over a stretch of the clip. Two keyframes,
    /// **linear**: a volume line between two points is exactly what an editor
    /// draws by dragging one, and inventing a curve nobody asked for is how a
    /// fade stops sounding like the one that was requested.
    ///
    /// Outside the ramp the value holds — `from` before it, `to` after — which
    /// is [`KeyframeTrack::value_at`]'s rule and not a special case here.
    Ramp {
        /// Where the value starts, and what it reads before the ramp begins.
        from: f64,
        /// Where it arrives, and what it reads for the rest of the clip.
        to: f64,
        /// When the ramp begins, in frames from the start of the clip.
        at: Frames,
        /// How long it takes. Never zero — see [`LevelError::Instant`].
        over: Frames,
    },
}

impl Level {
    /// The keyframes this level is made of.
    fn keyframes(self) -> Vec<Keyframe> {
        let point = |t: Frames, value: f64| Keyframe {
            t,
            value,
            easing: Easing::Linear,
        };
        match self {
            Self::Flat(value) => vec![point(Frames::ZERO, value)],
            Self::Ramp { from, to, at, over } => vec![point(at, from), point(at + over, to)],
        }
    }

    /// The last frame of the clip this level needs there to be.
    fn reaches(self) -> Frames {
        match self {
            Self::Flat(_) => Frames::ZERO,
            Self::Ramp { at, over, .. } => at + over,
        }
    }

    /// Refuses a level the format could not carry, before anything is touched.
    fn check(self) -> Result<(), LevelError> {
        match self {
            Self::Flat(value) => finite(value),
            Self::Ramp { from, to, over, .. } => {
                finite(from)?;
                finite(to)?;
                if over == Frames::ZERO {
                    return Err(LevelError::Instant);
                }
                Ok(())
            }
        }
    }
}

/// A value the format can hold. Non-finite is refused here rather than left to
/// validation, so the caller is told which number was the problem.
fn finite(value: f64) -> Result<(), LevelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LevelError::Value(value))
    }
}

/// What setting a level replaced.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Levelled {
    /// The tracks that animated this property before, in the order they sat in
    /// the document. Empty when the clip was not animating it at all.
    ///
    /// Handed back rather than counted, because *who wrote them* is the part a
    /// caller has to pass on: a dip signed by ducking is somebody else's work
    /// disappearing, and a hand-written fade is the user's own.
    pub replaced: Vec<KeyframeTrack>,
}

/// Why a level was not set. Nothing is ever partly applied.
#[derive(Debug, thiserror::Error)]
pub enum LevelError {
    /// No clip in this project carries that id.
    #[error("no clip `{0}` in this project")]
    NoClip(ClipId),
    /// A value the format cannot hold — infinity, or not a number.
    #[error("a level must be a finite number, and {0} is not")]
    Value(f64),
    /// A ramp of no frames. Two keyframes at one instant is not a document
    /// that loads, and what was meant by it is a flat level.
    #[error("a ramp of no frames is a jump — ask for a flat level instead")]
    Instant,
    /// The ramp ran off the end of the clip, so the value would never arrive.
    /// Refused rather than clamped: a fade that finishes after the last frame
    /// is a fade nobody hears the end of, and shortening it silently is a
    /// different edit from the one asked for.
    #[error("the ramp ends at frame {end} and `{clip}` is only {duration} frames long")]
    PastTheEnd {
        /// The clip the ramp did not fit inside.
        clip: ClipId,
        /// Where the ramp would have ended.
        end: Frames,
        /// How long the clip actually is.
        duration: Frames,
    },
    /// The result was not a document that loads.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Sets `property` on one clip to `level`, replacing whatever animated it
/// before.
///
/// All or nothing, and validated before it lands: the change is worked out on a
/// copy and only a copy [`Project::validate`] accepts becomes the document. A
/// refused level leaves `project` exactly as it was.
pub fn set(
    project: &mut Project,
    clip: &ClipId,
    property: &PropertyPath,
    level: Level,
) -> Result<Levelled, LevelError> {
    level.check()?;

    let mut proposed = project.clone();
    let found = proposed
        .tracks
        .iter_mut()
        .flat_map(|track| track.clips.iter_mut())
        .find(|candidate| &candidate.id == clip)
        .ok_or_else(|| LevelError::NoClip(clip.clone()))?;
    let end = level.reaches();
    if end > found.duration {
        return Err(LevelError::PastTheEnd {
            clip: clip.clone(),
            end,
            duration: found.duration,
        });
    }

    let levelled = Levelled {
        replaced: take_over(found, property, level),
    };
    proposed.validate()?;
    *project = proposed;
    Ok(levelled)
}

/// Puts the one track for `property` on the clip, and returns the ones it
/// displaced.
fn take_over(clip: &mut Clip, property: &PropertyPath, level: Level) -> Vec<KeyframeTrack> {
    let (replaced, kept): (Vec<_>, Vec<_>) = clip
        .keyframes
        .drain(..)
        .partition(|track| &track.property == property);
    clip.keyframes = kept;
    clip.keyframes
        .push(KeyframeTrack::new(property.clone(), level.keyframes()));
    replaced
}
