//! Scaling where clips sit in time, about an instant that stays put.
//!
//! The operation for **pacing**, which is most of what editing actually is. A
//! montage cut to a song that turned out to be 8% slower, a run of titles that
//! all need a beat more air, a sequence that drags in the middle — all of them
//! are "the same clips, spread differently", and every other way of doing it is
//! moving each clip in turn and getting the arithmetic slightly wrong.
//!
//! ```text
//!                    playhead
//!                       |
//!   before   [--A--]    |    [----B----]        [-C-]
//!   ×1.5  [--A--]       |        [----B----]              [-C-]
//!   ×0.7       [--A--]  |  [----B----]     [-C-]
//! ```
//!
//! ## Durations scale, and only where that is free
//!
//! Scaling starts alone is not a faster cut, it is the same cut with the gaps
//! squeezed out — seven four-second title cards sitting in a three-second
//! rhythm. So durations scale too, but **only for the clips whose duration is a
//! statement about the timeline and nothing else**: a still, a title, a colour,
//! a brief nobody has generated yet. See
//! [`Asset::has_intrinsic_duration`](crate::asset::Asset::has_intrinsic_duration).
//!
//! For a clip backed by recorded media the same instruction has two entirely
//! different readings — show less of the source, or play the whole of it faster
//! — and picking one silently is the failure mode, because the result *looks*
//! successful either way. Those clips move and keep their length, and [`Paced`]
//! names them so a caller can say so rather than let it be discovered.
//!
//! ## Rounding, and the property it buys
//!
//! Every position goes through one function and is rounded there and nowhere
//! else, so a clip's end and the next clip's start are rounded from the *same*
//! expression. That is what makes a gapless run stay gapless: with position and
//! duration both scaling by `k`, clip A's new end is exactly clip B's new
//! start, at every `k` and after rounding. A run that touched still touches.
//!
//! It is only a *mixed* selection — some clips scaling, some keeping a length
//! their media owns — that can open a gap or a collision. A collision is
//! refused by [`Project::validate`] like any other, having changed nothing, as
//! is a scale that would push a clip off the front of the timeline.
//!
//! Called through the module — `scorsese_core::pacing::scale` — rather than
//! re-exported at the crate root, because a bare `scale` there would read as
//! the spatial property of the same name.

use std::collections::BTreeSet;

use crate::project::Project;
use crate::time::Frames;
use crate::timeline::ClipId;
use crate::validate::ValidationErrors;

/// What a scale did, once the document accepted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paced {
    /// How many clips moved. Never zero — that is [`PaceError::Nothing`].
    pub moved: usize,
    /// The clips that moved but kept their length, because the media behind
    /// them has a length of its own. In document order.
    ///
    /// Reported rather than passed over in silence: this is the one place the
    /// operation does something other than what was asked, and a caller that
    /// cannot say which clips were left alone leaves the user to notice.
    pub kept: Vec<ClipId>,
}

/// Why a scale did not happen. Nothing is ever partly applied.
#[derive(Debug, thiserror::Error)]
pub enum PaceError {
    /// A factor that is not a positive, finite number. Zero collapses every
    /// clip onto the pivot and a negative one turns the edit back to front;
    /// neither is a thing anybody means by "spread these out".
    #[error("a scale factor must be a positive number, and {0} is not")]
    Factor(f64),
    /// None of the named clips is in this project.
    #[error("none of those clips is in this project")]
    Nothing,
    /// A clip would have landed before the first frame of the timeline. There
    /// is no time there, so the scale stops rather than pile clips up against
    /// frame zero — which would trim the ones that hit it, and a trim is not
    /// what anybody asked this for.
    #[error("`{clip}` would start before the beginning of the timeline")]
    BeforeTheStart {
        /// The clip that ran off the front.
        clip: ClipId,
    },
    /// A clip whose duration scales would have rounded away to nothing. A clip
    /// covering no frame renders nothing, so the whole scale stops here rather
    /// than quietly leaving a title one frame long.
    #[error("at ×{factor}, `{clip}` would be shorter than a single frame")]
    Vanishes {
        /// The clip that would have vanished.
        clip: ClipId,
        /// The factor that would have done it.
        factor: f64,
    },
    /// The result was not a document that loads — two clips sharing a frame of
    /// one track, most often, which is what a mixed selection can produce.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Moves `clips` toward or away from `about` by `factor`, scaling the duration
/// of each clip whose duration is the timeline's to decide.
///
/// All or nothing, and validated before it lands: the change is worked out on a
/// copy and only a copy that [`Project::validate`] accepts becomes the
/// document. A refused scale leaves `project` exactly as it was, which is what
/// lets a live gesture keep running — the clips simply stop moving.
pub fn scale(
    project: &mut Project,
    clips: &BTreeSet<ClipId>,
    about: Frames,
    factor: f64,
) -> Result<Paced, PaceError> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err(PaceError::Factor(factor));
    }
    let stretching = stretching(project, clips);

    let mut proposed = project.clone();
    let mut paced = Paced {
        moved: 0,
        kept: Vec::new(),
    };
    for track in &mut proposed.tracks {
        for clip in &mut track.clips {
            if !clips.contains(&clip.id) {
                continue;
            }
            paced.moved += 1;
            let off_the_front = || PaceError::BeforeTheStart {
                clip: clip.id.clone(),
            };
            let start = mapped(clip.start, about, factor).ok_or_else(off_the_front)?;
            if stretching.contains(&clip.id) {
                let end = mapped(clip.end(), about, factor).ok_or_else(off_the_front)?;
                let span = end.get() - start.get();
                if span == 0 {
                    return Err(PaceError::Vanishes {
                        clip: clip.id.clone(),
                        factor,
                    });
                }
                clip.duration = Frames(span);
            } else {
                paced.kept.push(clip.id.clone());
            }
            clip.start = start;
        }
        // A selected clip can pass an unselected one, so the array order a
        // reader depends on has to be restored. Nothing reads it — clips on a
        // track cannot overlap — but `project.json` is a document people and
        // agents read, and a track listed out of order is one nobody can
        // follow.
        track.clips.sort_by_key(|clip| clip.start);
    }
    if paced.moved == 0 {
        return Err(PaceError::Nothing);
    }
    proposed.validate()?;
    *project = proposed;
    Ok(paced)
}

/// Which of the selected clips have a duration the timeline owns.
///
/// An asset the document does not have is treated as media-backed, which is the
/// cautious reading: it keeps a length rather than inventing a rule for a clip
/// nothing can describe. A valid project has no such clip anyway.
fn stretching(project: &Project, clips: &BTreeSet<ClipId>) -> BTreeSet<ClipId> {
    project
        .clips()
        .filter(|(_, clip)| clips.contains(&clip.id))
        .filter(|(_, clip)| {
            project
                .asset(&clip.asset)
                .is_some_and(|asset| !asset.has_intrinsic_duration())
        })
        .map(|(_, clip)| clip.id.clone())
        .collect()
}

/// Where `t` lands when time around `about` is stretched by `factor`, or
/// `None` when that is before the start of the timeline.
///
/// The one place a position is rounded, which is the whole reason it is a
/// function: a clip's end and the next clip's start come out of the same
/// expression, so a cut that was exact stays exact.
///
/// `None` rather than a clamp at zero. Clamping would hold the start of a clip
/// at frame zero while its end went on scaling, which is not a move — it is a
/// trim, applied to whichever clips happened to reach the front.
fn mapped(t: Frames, about: Frames, factor: f64) -> Option<Frames> {
    let pivot = about.get() as f64;
    let moved = (pivot + (t.get() as f64 - pivot) * factor).round();
    (moved >= 0.0).then_some(Frames(moved as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pivot is the one instant that does not move — that is what "about"
    /// means, and it is why the playhead is where a person puts it first.
    #[test]
    fn the_instant_scaled_about_stays_exactly_where_it_is() {
        for factor in [0.25, 0.9, 1.0, 3.0] {
            assert_eq!(mapped(Frames(600), Frames(600), factor), Some(Frames(600)));
        }
    }

    #[test]
    fn a_position_moves_away_from_the_pivot_and_back_toward_it() {
        assert_eq!(mapped(Frames(500), Frames(300), 2.0), Some(Frames(700)));
        assert_eq!(mapped(Frames(100), Frames(300), 0.5), Some(Frames(200)));
        assert_eq!(mapped(Frames::ZERO, Frames(300), 1.0), Some(Frames::ZERO));
    }

    /// There is no time before frame zero, and holding a clip there while its
    /// far edge went on scaling would be a trim rather than a move.
    #[test]
    fn a_position_before_the_start_of_the_timeline_has_no_answer() {
        assert_eq!(mapped(Frames(10), Frames(500), 4.0), None);
    }

    /// To the nearest frame, not toward zero. Truncating would drag every clip
    /// on one side of the pivot a frame closer to it, which over a run of cuts
    /// is a rhythm quietly losing a frame per shot.
    #[test]
    fn a_position_lands_on_the_nearest_frame() {
        assert_eq!(mapped(Frames(10), Frames::ZERO, 1.14), Some(Frames(11)));
        assert_eq!(mapped(Frames(10), Frames::ZERO, 1.16), Some(Frames(12)));
    }
}
