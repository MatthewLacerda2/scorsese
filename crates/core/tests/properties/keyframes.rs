//! Evaluating a keyframe track, over any track and any frame of it.
//!
//! `value_at` is called once per animated property per frame, so everything
//! here is on the hot path of every render there is: a wrong answer is an
//! animation that arrives early, and a panic is a render that stops.

use proptest::prelude::*;
use scorsese_core::{Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

use crate::easing::any_easing;
use crate::runner::check;

/// This file, so a failure is written down beside it. See `runner`.
const SOURCE: &str = file!();

/// What a keyframe holds. Its meaning is the property's, never core's — so
/// the range is chosen for the arithmetic rather than for any property:
/// generous enough to cover an opacity, a volume and a position in pixels,
/// and short of the magnitudes where a *difference* of two finite values
/// overflows to infinity, which is a claim about `f64` and not about this
/// crate.
fn value() -> impl Strategy<Value = f64> {
    -1.0e6f64..=1.0e6
}

/// A well-formed track: times ascending, none repeated, which is what
/// validation guarantees and what the evaluator is entitled to assume.
fn track() -> impl Strategy<Value = KeyframeTrack> {
    prop::collection::btree_map(0u64..=100_000u64, (value(), any_easing()), 1..8).prop_map(points)
}

/// A frame to ask about: inside the keyframed span, or a long way outside it.
fn frame() -> impl Strategy<Value = Frames> {
    prop_oneof![3 => 0u64..=120_000u64, 1 => any::<u64>()].prop_map(Frames)
}

fn points(map: std::collections::BTreeMap<u64, (f64, Easing)>) -> KeyframeTrack {
    KeyframeTrack::new(
        // Nothing here mentions a real property name, and that is the point:
        // the same evaluator serves an opacity ramp, a move and an audio fade.
        PropertyPath::new("some.numeric.property"),
        map.into_iter()
            .map(|(t, (value, easing))| Keyframe {
                t: Frames(t),
                value,
                easing,
            })
            .collect(),
    )
}

#[test]
fn a_value_never_leaves_the_range_of_its_own_keyframes() {
    // Interpolation must not invent a value nobody wrote. This is the
    // property that catches an easing overshooting before it reaches a pixel,
    // and it is the reason the easing curves are held to their endpoints one
    // file over.
    check(SOURCE, (track(), frame()), |(track, t)| {
        let values = track.keyframes.iter().map(|k| k.value);
        let low = values.clone().fold(f64::INFINITY, f64::min);
        let high = values.fold(f64::NEG_INFINITY, f64::max);
        let at = track.value_at(t).expect("a track with keyframes answers");
        prop_assert!(
            at >= low && at <= high,
            "{at} at {t} is outside {low}..={high}"
        );
        Ok(())
    });
}

#[test]
fn a_keyframes_own_frame_reads_back_exactly() {
    // Exactly, not nearly. A keyframe is what an author wrote down; if the
    // evaluator returns something a hair off at the very frame it was placed
    // on, every value in the document has become approximate.
    check(SOURCE, track(), |track| {
        for keyframe in &track.keyframes {
            let at = track
                .value_at(keyframe.t)
                .expect("a track with keyframes answers");
            prop_assert_eq!(at, keyframe.value, "at {}", keyframe.t);
        }
        Ok(())
    });
}

#[test]
fn outside_the_keyframed_span_the_endpoint_holds() {
    // Extrapolating would have a two-keyframe fade keep darkening past the
    // end of the clip, which is not what two keyframes mean.
    check(SOURCE, (track(), any::<u64>()), |(track, away)| {
        let first = *track.keyframes.first().expect("at least one keyframe");
        let last = *track.keyframes.last().expect("at least one keyframe");
        let before = Frames(first.t.get().saturating_sub(away));
        let after = Frames(last.t.get().saturating_add(away));
        prop_assert_eq!(track.value_at(before), Some(first.value), "{} before", away);
        prop_assert_eq!(track.value_at(after), Some(last.value), "{} after", away);
        Ok(())
    });
}

#[test]
fn asking_for_any_frame_of_any_track_never_panics() {
    // Any track, including the ones validation would reject: times out of
    // order, times repeated, values at the ends of what an `f64` holds. A
    // panic in the evaluator is a crashed render, and nothing upstream of it
    // is entitled to assume the document was checked first.
    let loose = prop::collection::vec(
        (
            any::<u64>(),
            prop_oneof![value(), -1.0e308f64..=1.0e308],
            any_easing(),
        ),
        0..8,
    );
    check(SOURCE, (loose, any::<u64>()), |(points, t)| {
        let keyframes = points
            .into_iter()
            .map(|(t, value, easing)| Keyframe {
                t: Frames(t),
                value,
                easing,
            })
            .collect();
        let track = KeyframeTrack::new(PropertyPath::new("p"), keyframes);
        let answer = track.value_at(Frames(t));
        // A well-formed track always answers; an out-of-order one is
        // validation's problem, and all this claims about it is that asking
        // does not take the process with it.
        if track.is_sorted() {
            prop_assert_eq!(answer.is_some(), !track.keyframes.is_empty());
        }
        Ok(())
    });
}
