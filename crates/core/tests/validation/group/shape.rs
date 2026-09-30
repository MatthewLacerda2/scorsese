//! What makes a group drawable: a block of tracks, only on a group, with
//! something in it, all of it picture, and never itself.

use scorsese_core::{
    AssetField, AssetKind, AssetProblem, ClipId, Frames, GroupProblem as G, TimelineProblem as T,
    TrackId,
};
use serde_json::json;

use super::{base, group_tracks, load};
use crate::common::{assert_only_problem, asset_id, reports};

#[test]
fn a_group_needs_its_tracks_and_nothing_else_may_carry_them() {
    let mut missing = base();
    missing["assets"][3]
        .as_object_mut()
        .expect("an asset")
        .remove("group");
    missing["tracks"][1]["clips"] = json!([]);
    assert!(reports(
        &load(&missing),
        AssetProblem::MissingField {
            asset: asset_id("diagram"),
            field: AssetField::Group,
            kind: AssetKind::Group,
        }
    ));

    let mut stray = base();
    stray["assets"][0]["group"] = json!({ "tracks": [] });
    assert_only_problem(
        &load(&stray),
        AssetProblem::StrayField {
            asset: asset_id("bed"),
            field: AssetField::Group,
            kind: AssetKind::Color,
        },
    );
}

/// A layer that can never show anything looks exactly like a render that
/// failed, so an empty group is refused rather than drawn as nothing.
#[test]
fn an_empty_group_is_refused() {
    let mut document = base();
    document["assets"][3]["group"] = json!({ "tracks": [] });
    assert!(reports(
        &load(&document),
        G::Empty {
            asset: asset_id("diagram")
        }
    ));
}

#[test]
fn a_group_is_picture_only() {
    let mut document = base();
    group_tracks(&mut document).push(json!({ "id": "g-sound", "kind": "audio", "clips": [] }));
    assert_only_problem(
        &load(&document),
        G::SoundInGroup {
            asset: asset_id("diagram"),
            track: TrackId::new("g-sound"),
        },
    );
}

#[test]
fn a_group_may_not_contain_itself() {
    let mut document = base();
    group_tracks(&mut document).push(json!({ "id": "g-self", "kind": "video", "clips": [
        { "id": "c-self", "asset": "diagram", "start": 0, "duration": 30 } ] }));
    assert!(reports(
        &load(&document),
        G::ContainsItself {
            asset: asset_id("diagram")
        }
    ));
}

/// At any other rate a group frame falls between timeline frames, and so does
/// every keyframe inside it.
#[test]
fn a_group_clip_plays_at_one() {
    let mut document = base();
    document["tracks"][1]["clips"][0]["speed"] = json!(2.0);
    document["tracks"][1]["clips"][0]["duration"] = json!(15);
    assert_only_problem(
        &load(&document),
        G::AtSpeed {
            clip: ClipId::new("c-diagram"),
            asset: asset_id("diagram"),
            speed: 2.0,
        },
    );
}

/// A group's length is where its last member ends, and a clip of it may not
/// play past that — the ceiling footage has, derived instead of measured.
#[test]
fn a_group_clip_may_not_outlast_the_group() {
    let mut document = base();
    document["tracks"][1]["clips"][0]["duration"] = json!(40);
    document["tracks"][0]["clips"][0]["duration"] = json!(40);
    assert_only_problem(
        &load(&document),
        T::ClipOutlastsSource {
            clip: ClipId::new("c-diagram"),
            asset: asset_id("diagram"),
            reaches: Frames(40),
            available: Frames(30),
        },
    );
}
