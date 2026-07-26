//! A keyframe track nobody animates does not stop a render.
//!
//! The point of the whole feature is that it is a *signal*: the file that comes
//! out is exactly the file that would have come out anyway, and the only
//! difference is that the report says something.

use scorsese_core::Fps;
use scorsese_core::{Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};
use scorsese_render::{FrameRange, Note};

use crate::common::ffmpeg::{fixture_dir, inspect, tools};
use crate::common::{clip, project, video_track};
use crate::{colour_asset, render};

/// A clip animating `property`, whatever that turns out to mean.
fn animating(mut clip: Clip, property: &str) -> Clip {
    clip.keyframes.push(KeyframeTrack::new(
        PropertyPath::new(property),
        vec![
            Keyframe {
                t: Frames::ZERO,
                value: 0.0,
                easing: Easing::Linear,
            },
            Keyframe {
                t: Frames(15),
                value: 1.0,
                easing: Easing::Linear,
            },
        ],
    ));
    clip
}

#[test]
fn a_typo_warns_and_the_render_goes_ahead_anyway() {
    let tools = tools();
    let dir = fixture_dir("typo");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let project = project(
        vec![red],
        vec![video_track(
            "v1",
            // A fade that will never happen, because nothing reads `opactiy`.
            vec![animating(clip("c1", "red", 0, 15), "opactiy")],
        )],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(
        report.notes,
        [Note::UnknownProperty {
            clip: "c1".to_owned(),
            property: "opactiy".to_owned(),
            did_you_mean: Some("opacity"),
        }]
    );
    assert_eq!(
        inspect(&tools, &out).frames,
        15,
        "the render still happened"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_novel_property_is_no_reason_to_refuse_a_render() {
    // A project authored against a newer scorsese has to render on this one.
    // The warning says the track does nothing here; it does not say no.
    let tools = tools();
    let dir = fixture_dir("novel");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let project = project(
        vec![red],
        vec![video_track(
            "v1",
            vec![animating(clip("c1", "red", 0, 15), "transform.rotation")],
        )],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert!(
        matches!(
            report.notes.as_slice(),
            [Note::UnknownProperty {
                did_you_mean: None,
                ..
            }]
        ),
        "got {:?}",
        report.notes
    );
    assert_eq!(inspect(&tools, &out).frames, 15);
    std::fs::remove_dir_all(&dir).ok();
}
