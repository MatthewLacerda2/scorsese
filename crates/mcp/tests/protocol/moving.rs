//! Moving a clip to another track and removing clips, over the wire.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// A line of narration moved from its own lane onto the music's, later in the
/// cut — one edit, landing where it was asked to.
#[test]
fn a_clip_moves_to_another_track_of_its_kind_and_the_reply_says_where() {
    let dir = project("move");
    // The bed runs 0-600 on `music`; clear it first so the lane is free.
    let (text, failed) = said(&call(
        "clip_remove",
        json!({ "project": dir, "clips": ["m1"] }),
    ));
    assert!(!failed, "{text}");
    let (text, failed) = said(&call(
        "clip_move",
        json!({ "project": dir, "clip": "v1c", "track": "music", "start_seconds": 10.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("`v1c` moved from `vo` to `music`"),
        "got {text}"
    );
    assert!(text.contains("frame 300"), "got {text}");

    let document = read(&dir);
    assert_eq!(clips(&document, "music"), ["v1c"]);
    assert!(
        clips(&document, "vo").is_empty(),
        "it left the lane it was on"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The refusal `place_clip` gives for the same mistake: sound has no place on
/// a picture track.
#[test]
fn sound_onto_a_picture_track_writes_nothing() {
    let dir = project("move-kind");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "clip_move",
        json!({ "project": dir, "clip": "v1c", "track": "v1", "start_seconds": 30.0 }),
    ));
    assert!(failed, "narration is not picture");
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// Removing a clip leaves its asset and every other clip alone — no ripple.
#[test]
fn a_removed_clip_leaves_its_asset_and_its_neighbours_where_they_were() {
    let dir = project("remove");
    let (text, failed) = said(&call(
        "clip_remove",
        json!({ "project": dir, "clips": ["c1"] }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`c1` from `v1`"), "got {text}");

    let document = read(&dir);
    assert!(clips(&document, "v1").is_empty());
    assert_eq!(
        document["tracks"][2]["clips"][0]["start"], 60,
        "nothing moved"
    );
    assert!(
        document["assets"]
            .as_array()
            .expect("assets")
            .iter()
            .any(|asset| asset["id"] == "title"),
        "the asset stays"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// One mistyped id in the list is a list the caller has not read back.
#[test]
fn one_unknown_clip_removes_nothing_at_all() {
    let dir = project("remove-unknown");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "clip_remove",
        json!({ "project": dir, "clips": ["c1", "nope"] }),
    ));
    assert!(failed, "there is no clip `nope`");
    assert!(text.contains("`nope`"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

fn document(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("read")
}

fn read(dir: &std::path::Path) -> Value {
    serde_json::from_str(&document(dir)).expect("the server writes JSON")
}

/// The ids of the clips on one track, in order.
fn clips(project: &Value, track: &str) -> Vec<String> {
    project["tracks"]
        .as_array()
        .expect("tracks")
        .iter()
        .find(|held| held["id"] == track)
        .expect("the track is there")["clips"]
        .as_array()
        .expect("clips")
        .iter()
        .map(|clip| clip["id"].as_str().expect("an id").to_owned())
        .collect()
}
