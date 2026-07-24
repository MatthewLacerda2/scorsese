//! Keyframe shape — and, just as importantly, what is deliberately not
//! checked: whether a property path names a property that exists. Core
//! defines property types, never property values.

use crate::common::{assert_only_problem, clip_id, project};
use scorsese_core::{
    Easing, Frames, Keyframe, KeyframeTrack, Project, PropertyPath, ValidationError as E,
};

/// Replaces the `opacity` track on `c-logo` with these points.
fn set_opacity(project: &mut Project, keyframes: Vec<Keyframe>) {
    project.tracks[1].clips[0].keyframes[0] =
        KeyframeTrack::new(PropertyPath::new("opacity"), keyframes);
}

fn at(t: u64, value: f64) -> Keyframe {
    Keyframe {
        t: Frames(t),
        value,
        easing: Easing::Linear,
    }
}

fn opacity_problem(make: fn(clip: scorsese_core::ClipId, property: PropertyPath) -> E) -> E {
    make(clip_id("c-logo"), PropertyPath::new("opacity"))
}

#[test]
fn keyframes_must_ascend_in_time() {
    let mut p = project();
    set_opacity(&mut p, vec![at(30, 0.0), at(15, 1.0)]);
    assert_only_problem(
        &p,
        &opacity_problem(|clip, property| E::UnsortedKeyframes { clip, property }),
    );
}

#[test]
fn two_keyframes_on_the_same_frame_are_refused() {
    let mut p = project();
    set_opacity(&mut p, vec![at(15, 0.0), at(15, 1.0)]);
    assert_only_problem(
        &p,
        &opacity_problem(|clip, property| E::UnsortedKeyframes { clip, property }),
    );
}

#[test]
fn an_empty_keyframe_track_is_refused() {
    let mut p = project();
    set_opacity(&mut p, Vec::new());
    assert_only_problem(
        &p,
        &opacity_problem(|clip, property| E::EmptyKeyframeTrack { clip, property }),
    );
}

#[test]
fn a_keyframe_value_must_be_a_number() {
    let mut p = project();
    set_opacity(&mut p, vec![at(0, f64::NAN), at(30, 1.0)]);
    assert_only_problem(
        &p,
        &opacity_problem(|clip, property| E::BadKeyframeValue { clip, property }),
    );
}

#[test]
fn a_malformed_property_path_is_refused() {
    let mut p = project();
    p.tracks[1].clips[0].keyframes[0].property = PropertyPath::new("transform..x");
    assert_only_problem(
        &p,
        &E::MalformedPropertyPath {
            clip: clip_id("c-logo"),
            property: PropertyPath::new("transform..x"),
        },
    );
}

#[test]
fn a_property_the_compositor_has_never_heard_of_is_still_valid() {
    let mut p = project();
    p.tracks[1].clips[0].keyframes.push(KeyframeTrack::new(
        PropertyPath::new("wibble.wobble"),
        vec![at(0, 0.0), at(30, 1.0)],
    ));
    assert_eq!(
        p.validate(),
        Ok(()),
        "core must not gatekeep which properties exist"
    );
}

#[test]
fn easing_defaults_to_linear_when_omitted() {
    let project = project();
    let opacity = &project.tracks[1].clips[0].keyframes[0];
    assert_eq!(opacity.keyframes[0].easing, Easing::EaseIn);
    assert_eq!(opacity.keyframes[1].easing, Easing::Linear);
}

#[test]
fn keyframe_times_are_relative_to_the_clip_not_the_timeline() {
    // c-logo starts at frame 180 and its first keyframe is at frame 0 —
    // moving the clip must never require rewriting its keyframes.
    let project = project();
    let clip = &project.tracks[1].clips[0];
    assert_eq!(clip.start, Frames(180));
    assert_eq!(clip.keyframes[0].keyframes[0].t, Frames::ZERO);
}
