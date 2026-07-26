//! Keyframe tracks that name a property nothing animates.
//!
//! No ffmpeg anywhere in here: whether a property exists is a question about
//! the project and the code, and answering it should not need an encoder.

mod common;

use scorsese_core::{AssetKind, Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};
use scorsese_render::{ANIMATABLE, Unknown, unknown_in};

use common::{clip, file_asset, project, video_track};

/// A clip animating `property`, whatever that turns out to mean.
fn animating(id: &str, property: &str) -> Clip {
    let mut clip = clip(id, "a", 0, 30);
    clip.keyframes.push(KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value: 1.0,
            easing: Easing::Linear,
        }],
    ));
    clip
}

fn warnings_for(clips: Vec<Clip>) -> Vec<Unknown> {
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track("v1", clips)],
    );
    unknown_in(&project)
}

#[test]
fn the_vocabulary_here_is_the_compositors_and_the_mixers_together() {
    // Each list lives with the code that implements it, so this is the first
    // place both are visible. `volume` is the mixer's and `opacity` is the
    // compositor's, and a project does not have to care which is which.
    for known in [
        "opacity",
        "transform.position.x",
        "transform.scale.y",
        "volume",
    ] {
        assert!(
            ANIMATABLE.knows(&PropertyPath::new(known)),
            "`{known}` should be known to this build"
        );
    }
}

#[test]
fn a_project_animating_only_real_properties_has_nothing_said_about_it() {
    let clips = vec![animating("c1", "opacity"), animating("c2", "volume")];
    assert_eq!(warnings_for(clips), []);
}

#[test]
fn a_typo_names_the_clip_and_what_it_probably_meant() {
    // The case this exists for. Written `opactiy`, the track is perfectly
    // valid, the render succeeds, and the fade never happens.
    assert_eq!(
        warnings_for(vec![animating("c1", "opactiy")]),
        [Unknown {
            clip: "c1".to_owned(),
            property: "opactiy".to_owned(),
            did_you_mean: Some("opacity"),
        }]
    );
}

#[test]
fn a_genuinely_novel_property_is_reported_without_a_guess() {
    // A project authored against a newer scorsese is not making a mistake. It
    // is still worth mentioning that nothing here will act on it, but there is
    // nothing to suggest.
    assert_eq!(
        warnings_for(vec![animating("c1", "transform.rotation")]),
        [Unknown {
            clip: "c1".to_owned(),
            property: "transform.rotation".to_owned(),
            did_you_mean: None,
        }]
    );
}

#[test]
fn every_clip_with_one_is_named_rather_than_the_first() {
    // An agent repairing a project unattended fixes what it is told about, so
    // being told about one of three typos costs three round trips.
    let warnings = warnings_for(vec![
        animating("c1", "opactiy"),
        animating("c2", "opacity"),
        animating("c3", "volumme"),
    ]);
    let named: Vec<&str> = warnings.iter().map(|w| w.clip.as_str()).collect();
    assert_eq!(named, ["c1", "c3"]);
    assert_eq!(warnings[1].did_you_mean, Some("volume"));
}
