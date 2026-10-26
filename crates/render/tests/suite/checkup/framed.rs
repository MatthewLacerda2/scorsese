//! A title cut to fit and a clip running off the frame (#1096): warned about
//! once per clip, and quiet about a slide-in that has landed.

use scorsese_core::{
    Asset, AssetId, Clip, Easing, HashCheck, Keyframe, KeyframeTrack, Project, PropertyPath,
    TextStyle,
};
use scorsese_render::Checkup;
use scorsese_render::checkup::Severity;

use crate::common::{held, project, shape_asset, video_track};

/// Every warning about the frame's edges or about cut words.
fn about_edges(project: &Project) -> Vec<String> {
    let nowhere = std::env::temp_dir().join("scorsese-checkup-no-such-project.scor");
    Checkup::of(project, &nowhere, HashCheck::Skip)
        .lines()
        .iter()
        .filter(|line| line.says.contains("is cut off") || line.says.contains("runs off"))
        .inspect(|line| assert_eq!(line.severity, Severity::Warning, "{}", line.says))
        .map(|line| line.says.clone())
        .collect()
}

/// A half-width box on screen for 48 frames (1.6s at the fixture's 30), with `keyframes` on it.
fn moving(keyframes: Vec<KeyframeTrack>) -> Project {
    let clip = Clip {
        keyframes,
        ..Clip::at("badge", "panel", 0, 48)
    };
    project(
        vec![shape_asset("panel", 0.5, 0.25)],
        vec![video_track("v1", vec![clip])],
    )
}

#[test]
fn an_overlong_title_is_said_to_be_cut_once_however_long_it_plays() {
    let title = Asset {
        style: Some(TextStyle {
            size: 0.3,
            ..TextStyle::default()
        }),
        ..Asset::text(AssetId::new("title"), "ONE\nTWO\nTHREE\nFOUR")
    };
    let project = project(
        vec![title, shape_asset("panel", 0.1, 0.1)],
        vec![
            video_track("v1", vec![Clip::at("card", "title", 0, 60)]),
            // A cut on another track splits the title's stretch in two.
            video_track("v2", vec![Clip::at("dot", "panel", 20, 10)]),
        ],
    );
    let said = about_edges(&project);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].starts_with("text clip `card` is cut off"),
        "{}",
        said[0]
    );
}

#[test]
fn a_clip_held_half_off_the_frame_is_named_with_the_edge_and_the_instant() {
    let project = moving(Vec::new());
    let mut project = project;
    project.tracks[0].clips[0] = held(
        project.tracks[0].clips[0].clone(),
        "transform.position.x",
        0.5,
    );
    let said = about_edges(&project);
    assert_eq!(
        said,
        vec![
            "clip `badge` runs off the right of the frame by 25.0% of its width at 0.80s \
             (f24) — fine if it is meant to (a slide-in, a pan), otherwise move or shrink it"
                .to_owned()
        ]
    );
}

/// In from the right over its first half second, and centred by its middle:
/// a slide-in that landed is nothing to report.
#[test]
fn a_slide_in_that_has_landed_by_the_middle_says_nothing() {
    let key = |t: u64, value: f64| Keyframe {
        t: scorsese_core::Frames(t),
        value,
        easing: Easing::Linear,
    };
    let project = moving(vec![KeyframeTrack::new(
        PropertyPath::new("transform.position.x"),
        vec![key(0, 0.5), key(12, 0.0)],
    )]);
    assert_eq!(about_edges(&project), Vec::<String>::new());
}
