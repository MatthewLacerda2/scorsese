//! The document-wide lookups reach a group's members and lanes, and the
//! timeline-only ones do not (#1279).

use scorsese_core::grouping;
use scorsese_core::{AssetId, Project, TrackKind};

use super::{choose, diagram};

/// The diagram with both boxes and the arrow wrapped into `group`.
fn grouped() -> Project {
    let mut project = diagram();
    grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).expect("accepted");
    project
}

fn names<'a>(tracks: impl Iterator<Item = &'a scorsese_core::Track>) -> Vec<&'a str> {
    tracks.map(|track| track.id.as_str()).collect()
}

#[test]
fn the_stacks_are_the_timeline_then_each_group() {
    let project = grouped();
    let stacks: Vec<(Option<&str>, Vec<&str>)> = project
        .stacks()
        .map(|(group, tracks)| (group.map(AssetId::as_str), names(tracks.iter())))
        .collect();
    assert_eq!(
        stacks,
        [
            (None, names(project.tracks.iter())),
            (Some("group"), vec!["group-v2", "group-v3", "group-v4"]),
        ]
    );
}

#[test]
fn the_clips_of_an_asset_include_a_groups_members() {
    let project = grouped();
    let box_id = AssetId::new("box");
    let clips: Vec<&str> = project.clips_of(&box_id).map(|c| c.id.as_str()).collect();
    assert_eq!(clips, ["b1", "b2"]);
}

#[test]
fn a_new_track_id_is_free_of_a_groups_lanes_too() {
    let project = grouped();
    assert_eq!(project.free_track_id("group-v", 2).as_str(), "group-v5");
}

#[test]
fn the_tracks_of_a_kind_are_the_timelines_only() {
    let project = grouped();
    assert_eq!(
        names(project.tracks_of(TrackKind::Video)),
        names(project.tracks.iter())
    );
    assert_eq!(project.tracks_of(TrackKind::Audio).count(), 0);
}

#[test]
fn an_asset_is_looked_up_to_change() {
    let mut project = grouped();
    let bed = AssetId::new("bed");
    assert!(project.has_asset(&bed));
    assert!(!project.has_asset(&AssetId::new("nope")));
    project.asset_mut(&bed).expect("the bed").note = Some("dark".to_owned());
    assert_eq!(
        project.asset(&bed).and_then(|a| a.note.as_deref()),
        Some("dark")
    );
}
