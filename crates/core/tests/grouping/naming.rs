//! What a grouping is called and where it goes: the ids and the track asked
//! for, and the suffixing when the ones it would choose are taken.

use scorsese_core::grouping::{self, GroupError};
use scorsese_core::{Asset, AssetId, Clip, ClipId, Frames, Group, Rgba, Track, TrackId, TrackKind};

use crate::{choose, diagram};

fn track(id: &str, kind: TrackKind) -> Track {
    Track::new(TrackId::new(id), kind)
}

/// Every id asked for is used as given when it is free, and a clip id already
/// on the timeline is refused rather than taken twice.
#[test]
fn ids_asked_for_are_kept_and_a_taken_clip_id_is_refused() {
    let mut project = diagram();
    let mut named = choose(&["b1", "b2", "arrow"]);
    named.asset = Some(AssetId::new("chart"));
    named.clip = Some(ClipId::new("the-chart"));
    let grouped = grouping::group(&mut project, &named).unwrap();
    assert_eq!(grouped.asset.as_str(), "chart");
    assert_eq!(grouped.clip.as_str(), "the-chart");

    let mut project = diagram();
    named.clip = Some(ClipId::new("bg"));
    let error = grouping::group(&mut project, &named).unwrap_err();
    assert!(matches!(error, GroupError::TakenId { .. }), "got {error}");
    assert_eq!(project, diagram(), "nothing is written");
}

/// The group clip goes on the track asked for — here `v4`, which the arrow
/// leaves empty — and only a video track of the project will do.
#[test]
fn a_named_track_takes_the_group_clip_and_it_must_be_video() {
    let mut project = diagram();
    let mut named = choose(&["b1", "b2", "arrow"]);
    named.track = Some(TrackId::new("v4"));
    let grouped = grouping::group(&mut project, &named).unwrap();
    assert_eq!(grouped.track.as_str(), "v4");
    let on_v4: Vec<&str> = project.tracks[3]
        .clips
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(on_v4, ["c-group"]);

    for asked in ["a1", "nowhere"] {
        let mut project = diagram();
        project.tracks.push(track("a1", TrackKind::Audio));
        named.track = Some(TrackId::new(asked));
        let error = grouping::group(&mut project, &named).unwrap_err();
        assert!(
            matches!(error, GroupError::NoSuchTrack { .. }),
            "`{asked}` got {error}"
        );
    }
}

/// A document already holding the ids a grouping would choose: `group` to
/// `group-3` are assets, a timeline track and a track inside another group
/// have the lane names, and there are more tracks than clips and assets
/// together — an empty track is an ordinary thing to have. Each choice is
/// suffixed past what is taken, wherever in the document it was taken.
#[test]
fn ids_already_taken_anywhere_in_the_document_are_suffixed_past() {
    let mut project = diagram();
    let colour = |id: &str| Asset::color(AssetId::new(id), Rgba::WHITE);
    let mut inside = track("group-4-v3", TrackKind::Video);
    let clip = Clip::new(
        ClipId::new("inside"),
        AssetId::new("box"),
        Frames(0),
        Frames(10),
    );
    inside.clips.push(clip);
    let old = Group::new(vec![inside]);
    project.assets.extend([
        colour("group"),
        colour("group-2"),
        Asset::group(AssetId::new("group-3"), old),
    ]);
    project.tracks.push(track("group-4-v2", TrackKind::Video));
    for nth in 0..7 {
        project
            .tracks
            .push(track(&format!("empty-{nth}"), TrackKind::Video));
    }
    project.validate().expect("the crowded fixture is valid");

    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).unwrap();
    assert_eq!(grouped.asset.as_str(), "group-4");
    let asset = project.asset(&grouped.asset).expect("the new group");
    let group = asset.group.as_ref().expect("with its tracks");
    let lanes: Vec<&str> = group.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(lanes, ["group-4-v2-2", "group-4-v3-2", "group-4-v4"]);
}
