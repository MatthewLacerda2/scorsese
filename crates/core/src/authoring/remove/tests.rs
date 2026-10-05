//! Removing an asset or a lane: what goes, what is refused, and that a refusal
//! leaves the document as it was.

use std::collections::BTreeSet;

use super::{remove_asset, remove_track};
use crate::asset::AssetId;
use crate::authoring::AuthorError;
use crate::project::Project;
use crate::timeline::{ClipId, TrackId};

/// A caption shown twice on `v1` and once inside a group (beside a frame), a box an arrow is
/// attached to on `v2`, a card nothing shows, and an empty `a1`.
const DOCUMENT: &str = r##"{
  "schema_version": 44, "name": "T", "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "caption", "kind": "text", "text": "DAWN",
      "style": { "font": "serif", "size": 0.12 } },
    { "id": "card", "kind": "color", "color": "#101820" },
    { "id": "frame", "kind": "color", "color": "#202020" },
    { "id": "box", "kind": "shape", "shape": {
      "geometry": { "rectangle": { "width": 0.2, "height": 0.1 } },
      "fill": "#ffffffff" } },
    { "id": "arrow", "kind": "shape", "shape": {
      "geometry": { "arrow": {
        "from": { "attach": { "clip": "b", "side": "right" } },
        "to": { "x": 0.5, "y": 0.5 } } },
      "stroke": "#ffffffff", "stroke_width": 0.004 } },
    { "id": "inside", "kind": "group", "group": { "tracks": [
      { "id": "g1", "kind": "video", "clips": [
        { "id": "nested", "asset": "caption", "start": 0, "duration": 30 },
        { "id": "kept", "asset": "frame", "start": 30, "duration": 30 } ] } ] } }
  ],
  "tracks": [
    { "id": "v1", "kind": "video", "clips": [
      { "id": "first", "asset": "caption", "start": 0, "duration": 30 },
      { "id": "second", "asset": "caption", "start": 60, "duration": 30 } ] },
    { "id": "v2", "kind": "video", "clips": [
      { "id": "b", "asset": "box", "start": 0, "duration": 30 } ] },
    { "id": "a1", "kind": "audio", "clips": [] }
  ]
}"##;

fn project() -> Project {
    Project::from_json(DOCUMENT).expect("the fixture is a project")
}

fn named(ids: &[&str]) -> BTreeSet<ClipId> {
    ids.iter().map(|id| ClipId::new(*id)).collect()
}

#[test]
fn an_asset_nothing_shows_goes_with_an_empty_list() {
    let mut project = project();
    let removal = remove_asset(&mut project, &AssetId::new("card"), &named(&[])).expect("unused");
    assert_eq!(removal.asset.id.as_str(), "card");
    assert!(removal.clips.is_empty());
    assert!(
        !project
            .assets
            .iter()
            .any(|asset| asset.id.as_str() == "card")
    );
}

/// The refusal is the feature: it names every clip that would go, groups'
/// members included, so the next call can name them.
#[test]
fn an_asset_in_use_is_refused_until_its_clips_are_named() {
    let mut project = project();
    let before = project.clone();
    for wrong in [&[][..], &["first"], &["first", "second", "nested", "b"]] {
        let error = remove_asset(&mut project, &AssetId::new("caption"), &named(wrong))
            .expect_err("not exactly the clips that show it");
        let AuthorError::AssetInUse { using, .. } = &error else {
            panic!("got {error}");
        };
        assert_eq!(
            using,
            &vec![
                ClipId::new("first"),
                ClipId::new("nested"),
                ClipId::new("second")
            ]
        );
        assert!(error.to_string().contains("`nested`"), "{error}");
        assert_eq!(project, before, "nothing was removed");
    }
}

#[test]
fn named_exactly_the_asset_goes_with_every_clip_showing_it() {
    let mut project = project();
    let removal = remove_asset(
        &mut project,
        &AssetId::new("caption"),
        &named(&["second", "nested", "first"]),
    )
    .expect("named exactly");
    let went: Vec<(&str, &str)> = removal
        .clips
        .iter()
        .map(|gone| (gone.track.as_str(), gone.clip.id.as_str()))
        .collect();
    assert_eq!(went, [("v1", "first"), ("v1", "second"), ("g1", "nested")]);
    assert!(
        !project
            .every_clip()
            .any(|(_, clip)| clip.asset.as_str() == "caption")
    );
    assert!(
        !project
            .assets
            .iter()
            .any(|asset| asset.id.as_str() == "caption")
    );
    assert_eq!(project.tracks.len(), 3, "the lanes stay, emptied");
}

/// The box's clip is one an arrow is attached to, so taking it would leave
/// the arrow pointing at nothing — validation's refusal, not a partial removal.
#[test]
fn a_removal_validation_refuses_changes_nothing() {
    let mut project = project();
    let before = project.clone();
    let error = remove_asset(&mut project, &AssetId::new("box"), &named(&["b"]))
        .expect_err("the arrow needs `b`");
    assert!(matches!(error, AuthorError::Refused(_)), "got {error}");
    assert_eq!(project, before);
}

#[test]
fn an_unknown_asset_or_track_is_named_back() {
    let mut project = project();
    let asset = remove_asset(&mut project, &AssetId::new("nope"), &named(&[]));
    assert!(matches!(asset, Err(AuthorError::NoSuchAsset { .. })));
    let track = remove_track(&mut project, &TrackId::new("nope"), &named(&[]));
    assert!(matches!(track, Err(AuthorError::NoSuchTrack { .. })));
}

#[test]
fn an_empty_lane_goes_with_an_empty_list() {
    let mut project = project();
    let lane = remove_track(&mut project, &TrackId::new("a1"), &named(&[])).expect("empty");
    assert_eq!(lane.id.as_str(), "a1");
    assert_eq!(project.tracks.len(), 2);
}

#[test]
fn a_lane_with_clips_goes_only_with_them_named() {
    let mut project = project();
    let before = project.clone();
    let error = remove_track(&mut project, &TrackId::new("v1"), &named(&["first"]))
        .expect_err("`second` is on it too");
    assert!(
        matches!(error, AuthorError::TrackNotEmpty { .. }),
        "got {error}"
    );
    assert!(error.to_string().contains("`second`"), "{error}");
    assert_eq!(project, before);

    let lane = remove_track(
        &mut project,
        &TrackId::new("v1"),
        &named(&["first", "second"]),
    )
    .expect("named exactly");
    assert_eq!(
        lane.clips.len(),
        2,
        "the lane comes back with what was on it"
    );
    assert!(!project.every_track().any(|track| track.id.as_str() == "v1"));
    assert_eq!(project.assets.len(), 6, "the assets stay");
}

/// A group's lane is found where it lives — and this one is the group's only
/// lane, so taking it would leave a group of nothing, which validation refuses.
/// The same refusal stops an asset whose clips are all a group has.
#[test]
fn a_groups_lane_is_found_and_held_to_validation() {
    let mut project = project();
    let before = project.clone();
    let error = remove_track(
        &mut project,
        &TrackId::new("g1"),
        &named(&["nested", "kept"]),
    )
    .expect_err("the group would be empty");
    assert!(matches!(error, AuthorError::Refused(_)), "got {error}");
    assert_eq!(project, before);
}

mod sequence;
