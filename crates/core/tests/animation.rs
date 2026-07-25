//! Evaluating a keyframe track: the whole of the animation mechanism.
//!
//! Nothing here mentions a property name, and that is the point. The same
//! function has to serve an opacity ramp, a position move, and an audio fade,
//! because core defines property *types* and never property *values*.

use scorsese_core::{Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

fn track(points: &[(u64, f64, Easing)]) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new("some.numeric.property"),
        points
            .iter()
            .map(|&(t, value, easing)| Keyframe {
                t: Frames(t),
                value,
                easing,
            })
            .collect(),
    )
}

fn at(track: &KeyframeTrack, t: u64) -> f64 {
    track.value_at(Frames(t)).expect("a value")
}

#[test]
fn a_track_with_no_keyframes_says_nothing() {
    // Not zero: whatever reads this keeps its own default, rather than being
    // handed a value nobody wrote.
    assert_eq!(track(&[]).value_at(Frames(5)), None);
}

#[test]
fn a_single_keyframe_holds_for_all_time() {
    let single = track(&[(10, 0.4, Easing::Linear)]);
    assert_eq!(at(&single, 0), 0.4);
    assert_eq!(at(&single, 10), 0.4);
    assert_eq!(at(&single, 9_999), 0.4);
}

#[test]
fn outside_the_keyframed_span_the_value_holds() {
    // Extrapolating would have a two-keyframe fade keep darkening past the end
    // of the clip, which is not what two keyframes mean.
    let ramp = track(&[(10, 0.0, Easing::Linear), (20, 1.0, Easing::Linear)]);
    assert_eq!(at(&ramp, 0), 0.0, "before the first keyframe");
    assert_eq!(at(&ramp, 10), 0.0);
    assert_eq!(at(&ramp, 20), 1.0);
    assert_eq!(at(&ramp, 100), 1.0, "after the last keyframe");
}

#[test]
fn linear_travels_at_a_constant_rate() {
    let ramp = track(&[(0, 0.0, Easing::Linear), (10, 1.0, Easing::Linear)]);
    assert_eq!(at(&ramp, 2), 0.2);
    assert_eq!(at(&ramp, 5), 0.5);
    assert_eq!(at(&ramp, 8), 0.8);
}

#[test]
fn a_ramp_downwards_works_the_same_way() {
    let fade = track(&[(0, 1.0, Easing::Linear), (4, 0.0, Easing::Linear)]);
    assert_eq!(at(&fade, 1), 0.75);
    assert_eq!(at(&fade, 3), 0.25);
}

#[test]
fn easings_share_their_endpoints_and_differ_in_between() {
    let curves = [Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut];
    for easing in curves {
        let ramp = track(&[(0, 0.0, easing), (10, 1.0, Easing::Linear)]);
        assert_eq!(at(&ramp, 0), 0.0, "{easing:?} starts where it says");
        assert_eq!(at(&ramp, 10), 1.0, "{easing:?} ends where it says");
    }
    // Ease-in starts slow and ease-out starts fast; that is the whole of what
    // the names promise, and it is worth asserting rather than assuming.
    let ease_in = track(&[(0, 0.0, Easing::EaseIn), (10, 1.0, Easing::Linear)]);
    let ease_out = track(&[(0, 0.0, Easing::EaseOut), (10, 1.0, Easing::Linear)]);
    assert!(at(&ease_in, 2) < 0.2, "ease-in lags a linear ramp early");
    assert!(at(&ease_out, 2) > 0.2, "ease-out leads it");
    assert_eq!(at(&ease_in, 5), 0.25);
    assert_eq!(at(&ease_out, 5), 0.75);
}

#[test]
fn ease_in_out_is_symmetric_about_the_middle() {
    let ramp = track(&[(0, 0.0, Easing::EaseInOut), (10, 1.0, Easing::Linear)]);
    assert_eq!(at(&ramp, 5), 0.5);
    assert!((at(&ramp, 2) + at(&ramp, 8) - 1.0).abs() < 1e-9);
}

#[test]
fn hold_does_not_travel_at_all_and_then_jumps() {
    let stepped = track(&[(0, 0.2, Easing::Hold), (10, 0.9, Easing::Linear)]);
    assert_eq!(at(&stepped, 0), 0.2);
    assert_eq!(at(&stepped, 9), 0.2, "still holding one frame before");
    assert_eq!(at(&stepped, 10), 0.9, "and it jumps on arrival");
}

#[test]
fn each_segment_uses_the_easing_of_the_keyframe_it_leaves() {
    // Segment one holds, segment two ramps. An evaluator reading the easing off
    // the wrong end of the segment would get both of these backwards.
    let mixed = track(&[
        (0, 0.0, Easing::Hold),
        (10, 0.5, Easing::Linear),
        (20, 1.0, Easing::Linear),
    ]);
    assert_eq!(at(&mixed, 5), 0.0, "the first segment holds");
    assert_eq!(at(&mixed, 15), 0.75, "the second interpolates");
}
