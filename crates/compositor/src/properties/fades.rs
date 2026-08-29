//! The two fades, which are keyframes and nothing else.
//!
//! Sugar, and nothing but sugar: both write ordinary opacity keyframes, which
//! stay visible, editable and deletable like any others. There is no fade
//! machinery for a renderer to know about, which is why a fade composes with a
//! move or a zoom for free — and why these sit beside [`super::Properties`]
//! rather than inside it. Nothing here resolves anything; they *author*.

use scorsese_core::{Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

use super::path;

/// Ramps a clip up from nothing over its first `duration` frames.
///
/// Sugar, and nothing but sugar: it writes ordinary opacity keyframes, which
/// stay visible, editable, and deletable like any others. There is no fade
/// machinery for a renderer to know about, which is why a fade composes with a
/// move or a zoom for free.
///
/// Linear, because a fade is the neutral case and a curve is the author's
/// choice — edit the `easing` on the keyframe it writes.
pub fn fade_in(clip: &mut Clip, duration: Frames) {
    let duration = duration.get().min(clip.duration.get());
    if duration == 0 {
        return;
    }
    set_opacity(clip, Frames::ZERO, 0.0);
    set_opacity(clip, Frames(duration), 1.0);
}

/// Ramps a clip down to nothing over its last `duration` frames.
///
/// The ramp reaches zero at the clip's end — the frame after its last — so the
/// picture is still just barely there on the final frame and goes out exactly
/// on the cut.
pub fn fade_out(clip: &mut Clip, duration: Frames) {
    let total = clip.duration.get();
    let duration = duration.get().min(total);
    if duration == 0 {
        return;
    }
    set_opacity(clip, Frames(total - duration), 1.0);
    set_opacity(clip, Frames(total), 0.0);
}

/// Writes one opacity keyframe, replacing any already at that time and keeping
/// the track sorted — which validation requires and the evaluator assumes.
fn set_opacity(clip: &mut Clip, t: Frames, value: f64) {
    let track = match clip
        .keyframes
        .iter_mut()
        .find(|track| track.property.as_str() == path::OPACITY)
    {
        Some(track) => track,
        None => {
            clip.keyframes.push(KeyframeTrack::new(
                PropertyPath::new(path::OPACITY),
                Vec::new(),
            ));
            clip.keyframes
                .last_mut()
                .expect("the track just pushed is there")
        }
    };
    let keyframe = Keyframe {
        t,
        value,
        easing: Easing::Linear,
    };
    match track.keyframes.binary_search_by_key(&t, |frame| frame.t) {
        Ok(at) => track.keyframes[at] = keyframe,
        Err(at) => track.keyframes.insert(at, keyframe),
    }
}
