//! How many tracks one property may have on a clip (#1099): one, except where
//! the tracks multiply.

use crate::common::{assert_only_problem, clip_id, project};
use scorsese_core::{Easing, Frames, Keyframe, KeyframeTrack, PropertyPath, TimelineProblem as E};

fn track(property: &str) -> KeyframeTrack {
    let at = |t: u64, value| Keyframe {
        t: Frames(t),
        value,
        easing: Easing::Linear,
    };
    KeyframeTrack::new(PropertyPath::new(property), vec![at(0, 0.0), at(30, 1.0)])
}

#[test]
fn a_second_track_on_a_property_that_does_not_stack_is_refused_once() {
    let mut p = project();
    // `c-logo` already animates opacity; two more are still one problem.
    let keyframes = &mut p.tracks[1].clips[0].keyframes;
    keyframes.push(track("opacity").generated_by("dissolve"));
    keyframes.push(track("opacity"));
    assert_only_problem(
        &p,
        E::DuplicateKeyframeTrack {
            clip: clip_id("c-logo"),
            property: PropertyPath::new("opacity"),
        },
    );
}

#[test]
fn volume_tracks_stack() {
    let mut p = project();
    let keyframes = &mut p.tracks[1].clips[0].keyframes;
    keyframes.push(track("volume"));
    keyframes.push(track("volume").generated_by("duck"));
    keyframes.push(track("volume"));
    assert_eq!(p.validate(), Ok(()), "a fade and a duck multiply");
}

#[test]
fn one_track_on_each_of_two_properties_is_two_lanes_not_a_duplicate() {
    let mut p = project();
    p.tracks[1].clips[0]
        .keyframes
        .push(track("transform.scale.x"));
    assert_eq!(p.validate(), Ok(()));
}
