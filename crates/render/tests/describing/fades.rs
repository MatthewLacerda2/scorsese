//! Which way an animation goes, and when.
//!
//! "There is an opacity track on this clip" is not an answer to any question a
//! reader has. A fade that runs backwards, or lands half a second late, is
//! still a fade — so direction and timing are what these pin down.

use scorsese_core::{AssetKind, Easing, Frames};

use crate::common::{clip, file_asset, project, video_track};
use crate::{animated, described, ramp};

/// One two-second shot with `tracks` animated over it.
fn shot(tracks: Vec<scorsese_core::KeyframeTrack>) -> scorsese_core::Project {
    project(
        vec![file_asset("one", AssetKind::Video)],
        vec![video_track(
            "v1",
            vec![animated(clip("c1", "one", 0, 60), tracks)],
        )],
    )
}

#[test]
fn a_fade_out_says_which_way_it_goes_and_over_which_half_second() {
    // Opaque until frame 45, gone by frame 60 — the last half second.
    let description = described(&shot(vec![ramp(
        "opacity",
        [(45, 1.0), (60, 0.0)],
        Easing::Linear,
    )]));
    let animated = &description.stretches[0].picture[0].animated[0];

    assert_eq!(animated.property, "opacity");
    assert_eq!(
        (animated.at_start, animated.at_end),
        (1.0, 0.0),
        "solid at the top of the stretch, gone at the bottom"
    );
    let travel = animated.travel.expect("a fade travels");
    assert_eq!(
        (travel.from.frame, travel.to.frame),
        (Frames(45), Frames(60))
    );
    assert_eq!(
        (travel.from.seconds, travel.to.seconds),
        (1.5, 2.0),
        "the last half second, not the whole clip"
    );
    assert_eq!(travel.easing, Some(Easing::Linear));
}

/// The same shape backwards. If a description reported only *that* opacity is
/// animated, this and the test above would be indistinguishable.
#[test]
fn a_fade_in_is_not_a_fade_out() {
    let description = described(&shot(vec![ramp(
        "opacity",
        [(0, 0.0), (15, 1.0)],
        Easing::EaseIn,
    )]));
    let animated = &description.stretches[0].picture[0].animated[0];

    assert_eq!((animated.at_start, animated.at_end), (0.0, 1.0));
    let travel = animated.travel.expect("a fade travels");
    assert_eq!((travel.from.seconds, travel.to.seconds), (0.0, 0.5));
    assert_eq!(travel.easing, Some(Easing::EaseIn));
    assert!(
        format!("{description}").contains("opacity 0.00 → 1.00 over 0.00–0.50s (f0–15, ease_in)"),
        "and it reads that way too:\n{description}"
    );
}

/// A property whose keyframes all say the same thing is a value, not an
/// animation. Reporting it as travelling would make every static clip look like
/// it was moving.
#[test]
fn a_property_that_never_moves_is_reported_as_held() {
    let description = described(&shot(vec![ramp(
        "opacity",
        [(0, 0.5), (60, 0.5)],
        Easing::Linear,
    )]));
    let animated = &description.stretches[0].picture[0].animated[0];

    assert_eq!((animated.at_start, animated.at_end), (0.5, 0.5));
    assert_eq!(animated.travel, None);
    assert!(format!("{description}").contains("opacity 0.50 held"));
}

/// A fade that straddles a cut is reported inside each stretch it touches, cut
/// where the stretch is — so the times printed are always times of the stretch
/// they appear under.
#[test]
fn a_ramp_crossing_a_boundary_is_clipped_to_each_stretch() {
    let project = project(
        vec![
            file_asset("one", AssetKind::Video),
            file_asset("two", AssetKind::Video),
        ],
        vec![
            video_track(
                "v1",
                vec![animated(
                    clip("c1", "one", 0, 60),
                    vec![ramp("opacity", [(0, 0.0), (60, 1.0)], Easing::Linear)],
                )],
            ),
            video_track("v2", vec![clip("c2", "two", 30, 30)]),
        ],
    );

    let description = described(&project);
    let travel = |stretch: usize| {
        description.stretches[stretch].picture[0].animated[0]
            .travel
            .expect("the ramp is under way")
    };

    assert_eq!(
        (travel(0).from.frame, travel(0).to.frame),
        (Frames(0), Frames(30))
    );
    assert_eq!(
        (travel(1).from.frame, travel(1).to.frame),
        (Frames(30), Frames(60))
    );
    assert_eq!(
        description.stretches[1].picture[0].animated[0].at_start,
        0.5
    );
}

/// A keyframe track nothing in this build reads will never do anything, and
/// describing it would be asserting a fade that does not happen. The render
/// report already warns about the typo.
#[test]
fn a_property_nothing_animates_is_left_out() {
    let description = described(&shot(vec![ramp(
        "opactiy",
        [(0, 0.0), (30, 1.0)],
        Easing::Linear,
    )]));

    assert!(description.stretches[0].picture[0].animated.is_empty());
}
