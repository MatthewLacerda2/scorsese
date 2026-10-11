//! Keyframes riding along with a clip whose duration scaled.
//!
//! A keyframe's `t` counts from the start of its clip, which is what lets a
//! clip move without anything rewriting its animation — and exactly why a clip
//! that *stretches* has to have it rewritten. A title fading out over its last
//! half-second, scaled ×1.5, would otherwise finish the fade a third of the way
//! before its new end and sit there invisible; scaled ×0.5, it would end before
//! the fade began. So the points scale with the clip, by the same ratio, and a
//! fade at the end of a clip is still at the end of it.
//!
//! **Only what was written by hand.** A track a tool signed was worked out by
//! that tool from where things were, and the honest repair is the tool's own:
//! run it again. Rescaling its points would produce numbers that look right and
//! are not what the tool would say. Those tracks are left exactly as they were
//! and the tool is named, so a caller can say which to re-run.

use std::collections::BTreeSet;

use super::PaceError;
use crate::time::Frames;
use crate::timeline::Clip;

/// Scales `clip`'s hand-written keyframes from its length `before` to the one
/// it has now. `true` when there were any to scale.
///
/// Each time is rounded to the nearest frame. Two points that round onto one
/// frame would no longer be a track that loads, and dropping either would be
/// this function deciding which of somebody's keyframes to lose, so the scale
/// is refused instead — at that factor, on that clip, and nothing is changed.
pub(super) fn rescale(clip: &mut Clip, before: Frames, factor: f64) -> Result<bool, PaceError> {
    let (from, to) = (before.get(), clip.duration.get());
    if from == 0 || from == to {
        return Ok(false);
    }
    let ratio = to as f64 / from as f64;
    let mut any = false;
    for track in clip.keyframes.iter_mut().filter(|track| track.by.is_none()) {
        let sorted = track.is_sorted();
        for key in &mut track.keyframes {
            key.t = Frames((key.t.get() as f64 * ratio).round() as u64);
            any = true;
        }
        if sorted && !track.is_sorted() {
            return Err(PaceError::Crowded {
                clip: clip.id.clone(),
                property: track.property.clone(),
                factor,
            });
        }
    }
    Ok(any)
}

/// The tools whose signed tracks sit on `clip`, and so were left where they
/// were — timed from positions this scale has just changed.
pub(super) fn signed(clip: &Clip, tools: &mut BTreeSet<String>) {
    tools.extend(clip.keyframes.iter().filter_map(|track| track.by.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetId;
    use crate::keyframe::{Keyframe, KeyframeTrack, PropertyPath};
    use crate::timeline::ClipId;

    fn faded(times: &[u64], tool: Option<&str>) -> Clip {
        let mut clip = Clip::new(
            ClipId::new("t"),
            AssetId::new("a"),
            Frames::ZERO,
            Frames(120),
        );
        let keys = times
            .iter()
            .map(|&t| Keyframe {
                t: Frames(t),
                value: 1.0,
                easing: Default::default(),
            })
            .collect();
        let track = KeyframeTrack::new(PropertyPath::new("opacity"), keys);
        clip.keyframes
            .push(tool.map_or(track.clone(), |tool| track.generated_by(tool)));
        clip
    }

    fn times(clip: &Clip) -> Vec<u64> {
        clip.keyframes[0]
            .keyframes
            .iter()
            .map(|k| k.t.get())
            .collect()
    }

    #[test]
    fn a_fade_out_ends_where_the_clip_now_ends() {
        let mut clip = faded(&[0, 105, 120], None);
        clip.duration = Frames(180);
        assert!(rescale(&mut clip, Frames(120), 1.5).unwrap());
        assert_eq!(times(&clip), [0, 158, 180]);
    }

    #[test]
    fn a_clip_whose_length_did_not_change_keeps_its_points() {
        let mut clip = faded(&[0, 105, 120], None);
        assert!(!rescale(&mut clip, Frames(120), 1.0).unwrap());
        assert_eq!(times(&clip), [0, 105, 120]);
    }

    /// A tool's track is the tool's to redo; it is named, not touched.
    #[test]
    fn a_signed_track_is_left_alone_and_its_tool_named() {
        let mut clip = faded(&[0, 105, 120], Some("caption"));
        clip.duration = Frames(60);
        assert!(!rescale(&mut clip, Frames(120), 0.5).unwrap());
        assert_eq!(times(&clip), [0, 105, 120]);
        let mut tools = BTreeSet::new();
        signed(&clip, &mut tools);
        assert_eq!(tools.into_iter().collect::<Vec<_>>(), ["caption"]);
    }

    /// Points that would share a frame refuse the scale rather than lose one.
    #[test]
    fn points_squeezed_onto_one_frame_refuse_the_scale() {
        let mut clip = faded(&[0, 1, 2], None);
        clip.duration = Frames(12);
        let refused = rescale(&mut clip, Frames(120), 0.1);
        assert!(matches!(refused, Err(PaceError::Crowded { .. })));
    }
}
