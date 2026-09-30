//! A clip that follows an arrow and never moves along it.
//!
//! It renders — at the arrow's tail, for the whole clip — so nothing else would
//! ever say so; the checkup is the one place that can.

use scorsese_core::{
    Asset, AssetId, ClipId, Follow, HashCheck, Heads, Point, Project, Rgba, Shape,
};
use scorsese_core::{Easing, Frames, Geometry, Keyframe, KeyframeTrack, PropertyPath};
use scorsese_render::Checkup;
use scorsese_render::checkup::Severity;

use crate::common::{clip, project, shape_asset, video_track};

/// A dot following a line, with `progress` keyframed or not.
fn following(progress: bool) -> Project {
    let geometry = Geometry::Arrow {
        from: Point::new(0.1, 0.5).into(),
        to: Point::new(0.9, 0.5).into(),
        curve: Default::default(),
        heads: Heads::End,
    };
    let line = Asset::shape(AssetId::new("line"), Shape::outlined(geometry, Rgba::WHITE));
    let mut dot = clip("c-dot", "dot", 0, 30);
    dot.follow = Some(Follow::new(ClipId::new("c-line")));
    if progress {
        let key = |t, value| Keyframe {
            t: Frames(t),
            value,
            easing: Easing::Linear,
        };
        dot.keyframes.push(KeyframeTrack::new(
            PropertyPath::new("follow.progress"),
            vec![key(0, 0.0), key(29, 1.0)],
        ));
    }
    project(
        vec![line, shape_asset("dot", 0.05, 0.05)],
        vec![
            video_track("v1", vec![clip("c-line", "line", 0, 30)]),
            video_track("v2", vec![dot]),
        ],
    )
}

fn warnings(project: &Project) -> Vec<String> {
    let dir = std::env::temp_dir().join("scorsese-checkup-following.scor");
    Checkup::of(project, &dir, HashCheck::Skip)
        .lines()
        .iter()
        .filter(|line| line.severity == Severity::Warning)
        .map(|line| line.says.clone())
        .collect()
}

#[test]
fn a_follower_that_never_keyframes_its_progress_is_warned_about() {
    let still = following(false);
    still.validate().expect("a valid document: it renders");
    let said = warnings(&still);
    let about = said
        .iter()
        .filter(|line| line.contains("`c-dot` follows arrow `c-line`"));
    assert_eq!(about.count(), 1, "{said:?}");
}

#[test]
fn a_follower_that_moves_is_not() {
    let said = warnings(&following(true));
    assert!(!said.iter().any(|line| line.contains("c-dot")), "{said:?}");
}
