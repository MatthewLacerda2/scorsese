//! Moving a video lane under or over another.

use super::*;
use crate::authoring::{Lane, add_track};
use crate::time::Fps;

/// A timeline of `v1`, `a1`, `v2`, `v3`, bottom to top.
fn project() -> Project {
    let mut project = Project::new("T", Fps::new(30, 1).expect("30fps"));
    for kind in [
        TrackKind::Video,
        TrackKind::Audio,
        TrackKind::Video,
        TrackKind::Video,
    ] {
        add_track(&mut project, &Lane::of(kind)).expect("a lane");
    }
    project
}

fn order(project: &Project) -> Vec<&str> {
    project.tracks.iter().map(|lane| lane.id.as_str()).collect()
}

fn id(name: &str) -> TrackId {
    TrackId::new(name)
}

#[test]
fn a_lane_goes_directly_under_or_over_the_one_named() {
    let mut project = project();
    move_track(&mut project, &id("v3"), &Layer::Under(id("v1"))).expect("under");
    assert_eq!(order(&project), ["v3", "v1", "a1", "v2"]);
    move_track(&mut project, &id("v3"), &Layer::Over(id("v2"))).expect("over");
    assert_eq!(order(&project), ["v1", "a1", "v2", "v3"]);
    move_track(&mut project, &id("v1"), &Layer::Over(id("v2"))).expect("over, upward");
    assert_eq!(order(&project), ["a1", "v2", "v1", "v3"]);
}

#[test]
fn a_new_lane_can_be_added_under_another() {
    let mut project = project();
    let lane = Lane {
        layer: Some(Layer::Under(id("v2"))),
        ..Lane::of(TrackKind::Video)
    };
    let added = add_track(&mut project, &lane).expect("under v2");
    assert_eq!(added.as_str(), "v4");
    assert_eq!(order(&project), ["v1", "a1", "v4", "v2", "v3"]);
}

#[test]
fn what_has_no_layer_is_refused_and_moves_nothing() {
    let mut project = project();
    let before = project.clone();
    let refused = |project: &mut Project, track: &str, to: Layer| {
        move_track(project, &id(track), &to).expect_err("refused")
    };
    let error = refused(&mut project, "a1", Layer::Under(id("v1")));
    assert!(matches!(error, AuthorError::NotALayer { .. }), "{error}");
    let error = refused(&mut project, "v1", Layer::Over(id("a1")));
    assert!(matches!(error, AuthorError::NotALayer { .. }), "{error}");
    let error = refused(&mut project, "v1", Layer::Over(id("v1")));
    assert!(matches!(error, AuthorError::OwnLayer { .. }), "{error}");
    let error = refused(&mut project, "v9", Layer::Over(id("v1")));
    assert!(matches!(error, AuthorError::NoSuchTrack { ref track } if track.as_str() == "v9"));
    let error = refused(&mut project, "v1", Layer::Over(id("v9")));
    assert!(matches!(error, AuthorError::NoSuchTrack { ref track } if track.as_str() == "v9"));
    let audio = Lane {
        layer: Some(Layer::Under(id("v1"))),
        ..Lane::of(TrackKind::Audio)
    };
    let error = add_track(&mut project, &audio).expect_err("audio has no layer");
    assert!(matches!(error, AuthorError::NotALayer { .. }), "{error}");
    assert_eq!(project, before);
}

#[test]
fn a_groups_lanes_move_among_themselves_and_not_onto_the_timeline() {
    use crate::asset::{Asset, AssetId};
    use crate::color::Rgba;
    use crate::group::Group;
    use crate::time::Frames;
    use crate::timeline::{Clip, ClipId};

    let mut project = project();
    let mut lanes = ["g1", "g2"].map(|name| Track::new(id(name), TrackKind::Video));
    let card = AssetId::new("card");
    project.assets.push(Asset::color(card.clone(), Rgba::BLACK));
    let clip = Clip::new(ClipId::new("in"), card, Frames::ZERO, Frames(30));
    lanes[0].clips.push(clip);
    let group = Asset::group(AssetId::new("g"), Group::new(lanes.to_vec()));
    project.assets.push(group);
    move_track(&mut project, &id("g2"), &Layer::Under(id("g1"))).expect("within the group");
    let group = project.assets[1].group.as_ref().expect("the group");
    let ids: Vec<&str> = group.tracks.iter().map(|lane| lane.id.as_str()).collect();
    assert_eq!(ids, ["g2", "g1"]);
    let error = move_track(&mut project, &id("g1"), &Layer::Over(id("v1"))).expect_err("apart");
    assert!(matches!(error, AuthorError::ApartLayers { .. }), "{error}");
}
