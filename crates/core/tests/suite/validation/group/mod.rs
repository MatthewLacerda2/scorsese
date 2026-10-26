//! Groups: what makes one drawable, and what may reach across its edge.
//!
//! Every test starts from one small valid document — a box and an arrow
//! attached to it, grouped, over a background — and makes one change to it.

mod crossing;
mod shape;

use scorsese_core::{Project, SCHEMA_VERSION};
use serde_json::{Value, json};

/// The document every test here mutates: a group `diagram` holding a box and
/// an arrow following it, shown for 30 frames by clip `c-diagram` over a
/// background.
fn base() -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "name": "groups",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "bed", "kind": "color", "color": "#101820" },
            { "id": "box", "kind": "shape", "shape": {
                "geometry": { "rectangle": { "width": 0.2, "height": 0.1 } },
                "fill": "#ffffffff" } },
            { "id": "link", "kind": "shape", "shape": {
                "geometry": { "arrow": {
                    "from": { "x": 0.1, "y": 0.1 },
                    "to": { "attach": { "clip": "c-box", "side": "left" } } } },
                "stroke": "#ffffffff", "stroke_width": 0.004 } },
            { "id": "diagram", "kind": "group", "group": { "tracks": [
                { "id": "g-boxes", "kind": "video", "clips": [
                    { "id": "c-box", "asset": "box", "start": 0, "duration": 30 } ] },
                { "id": "g-links", "kind": "video", "clips": [
                    { "id": "c-link", "asset": "link", "start": 0, "duration": 30 } ] }
            ] } }
        ],
        "tracks": [
            { "id": "v1", "kind": "video", "clips": [
                { "id": "c-bed", "asset": "bed", "start": 0, "duration": 30 } ] },
            { "id": "v2", "kind": "video", "clips": [
                { "id": "c-diagram", "asset": "diagram", "start": 0, "duration": 30 } ] }
        ]
    })
}

/// A document as a project, parsed but not validated.
fn load(document: &Value) -> Project {
    Project::from_json(&document.to_string()).expect("the document parses")
}

/// The group's tracks, to mutate.
fn group_tracks(document: &mut Value) -> &mut Vec<Value> {
    document["assets"][3]["group"]["tracks"]
        .as_array_mut()
        .expect("the group has tracks")
}

#[test]
fn a_group_with_an_arrow_following_a_member_is_valid() {
    assert_eq!(load(&base()).validate(), Ok(()));
}
