//! What may reach across a group's edge: ids are one namespace for the whole
//! document, a member's asset resolves like any clip's, and an arrow follows
//! only clips on its own side.

use scorsese_core::{ClipId, GroupProblem as G, TimelineProblem as T};
use serde_json::json;

use super::{base, group_tracks, load};
use crate::common::{assert_only_problem, asset_id};

/// Arrows name clips by id, so a member's id must not also name a clip on the
/// timeline — or inside another group.
#[test]
fn clip_ids_are_unique_across_the_whole_document() {
    let mut document = base();
    document["tracks"][0]["clips"][0]["id"] = json!("c-box");
    assert_only_problem(
        &load(&document),
        T::DuplicateClipId {
            id: ClipId::new("c-box"),
        },
    );
}

#[test]
fn a_member_names_an_asset_the_table_has() {
    let mut document = base();
    group_tracks(&mut document)[0]["clips"][0]["asset"] = json!("nothing");
    assert_only_problem(
        &load(&document),
        T::DanglingAssetRef {
            clip: ClipId::new("c-box"),
            asset: asset_id("nothing"),
        },
    );
}

/// Outside a group an arrow is drawn on the frame; inside, before the group's
/// own transform. One following the other across the edge would be drawn in one
/// space and aimed in the other.
#[test]
fn an_arrow_outside_may_not_follow_a_member() {
    let mut document = base();
    group_tracks(&mut document).remove(1);
    document["tracks"]
        .as_array_mut()
        .expect("tracks")
        .push(json!({ "id": "v3", "kind": "video", "clips": [
            { "id": "c-link", "asset": "link", "start": 0, "duration": 30 } ] }));
    assert_only_problem(
        &load(&document),
        G::AttachedAcross {
            clip: ClipId::new("c-link"),
            arrow: asset_id("link"),
            target: ClipId::new("c-box"),
        },
    );
}

#[test]
fn an_arrow_inside_may_not_follow_a_clip_outside() {
    let mut document = base();
    document["assets"][2]["shape"]["geometry"]["arrow"]["to"]["attach"]["clip"] = json!("c-bed");
    assert_only_problem(
        &load(&document),
        G::AttachedAcross {
            clip: ClipId::new("c-link"),
            arrow: asset_id("link"),
            target: ClipId::new("c-bed"),
        },
    );
}

/// The group clip itself is on the frame like any other clip, so an arrow on
/// the frame may follow *it* — the way to point at a whole diagram.
#[test]
fn an_arrow_outside_may_follow_the_group_clip() {
    let mut document = base();
    document["assets"][2]["shape"]["geometry"]["arrow"]["to"]["attach"]["clip"] =
        json!("c-diagram");
    group_tracks(&mut document).remove(1);
    document["tracks"]
        .as_array_mut()
        .expect("tracks")
        .push(json!({ "id": "v3", "kind": "video", "clips": [
            { "id": "c-link", "asset": "link", "start": 0, "duration": 30 } ] }));
    assert_eq!(load(&document).validate(), Ok(()));
}
