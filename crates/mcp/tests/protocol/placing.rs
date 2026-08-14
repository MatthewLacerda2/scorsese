//! Placing a clip and trimming one, over the wire.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// Seconds in, frames on the grid out — and the reply says both, because a
/// caller that cannot read the frame back cannot tell where its cut landed.
#[test]
fn placing_a_clip_writes_it_at_the_frame_the_seconds_round_to() {
    let dir = project("place");
    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "title", "track": "v1",
                "start_seconds": 25.0, "duration_seconds": 2.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`title` placed on `v1`"), "got {text}");
    assert!(text.contains("frame 750"), "got {text}");

    assert_eq!(shape(&read(&dir), "v1", 1), (750, 60));
    std::fs::remove_dir_all(dir).ok();
}

/// The refusal the tool exists for. Frames 0-600 of `v1` are taken, and a
/// document with two clips over one instant of a track does not load.
#[test]
fn a_clip_landing_on_one_already_there_writes_nothing() {
    let dir = project("place-overlap");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "title", "track": "v1",
                "start_seconds": 5.0, "duration_seconds": 2.0 }),
    ));
    assert!(failed, "frames 150-210 of `v1` are `c1`'s");
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// A title has no length of its own, so there is no rest of the source to
/// take — and guessing one is how a card sits on screen for a length nobody
/// chose.
#[test]
fn an_omitted_duration_on_an_unmeasured_asset_is_refused_by_name() {
    let dir = project("place-unmeasured");
    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "title", "track": "v1", "start_seconds": 25.0 }),
    ));
    assert!(failed, "a text asset has no measured length");
    assert!(text.contains("no measured length"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A start on its own moves the clip and leaves its length alone: the line of
/// narration lands on the beat and goes on being the same line.
#[test]
fn trimming_a_start_moves_the_clip_and_keeps_its_length() {
    let dir = project("trim");
    let (text, failed) = said(&call(
        "trim_clip",
        json!({ "project": dir, "clip": "v1c", "start_seconds": 3.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`v1c` now starts at 3.00s"), "got {text}");

    assert_eq!(shape(&read(&dir), "vo", 0), (90, 90));
    std::fs::remove_dir_all(dir).ok();
}

/// A trim that names no field is a mistake, not a successful no-op.
#[test]
fn a_trim_that_asks_for_nothing_says_so() {
    let dir = project("trim-empty");
    let (text, failed) = said(&call("trim_clip", json!({ "project": dir, "clip": "v1c" })));
    assert!(failed, "nothing was named");
    assert!(text.contains("nothing to change"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// There is no time before frame zero, and rounding one up to it would move a
/// clip somewhere nobody asked for.
#[test]
fn a_negative_time_is_refused_rather_than_clamped() {
    let dir = project("trim-negative");
    let (text, failed) = said(&call(
        "trim_clip",
        json!({ "project": dir, "clip": "v1c", "start_seconds": -2.0 }),
    ));
    assert!(failed, "the timeline starts at 0s");
    assert!(text.contains("cannot be negative"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

fn document(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("read")
}

fn read(dir: &std::path::Path) -> Value {
    serde_json::from_str(&document(dir)).expect("the server writes JSON")
}

/// One clip's `(start, duration)`, by track id and position on it.
fn shape(project: &Value, track: &str, at: usize) -> (u64, u64) {
    let tracks = project["tracks"].as_array().expect("tracks");
    let found = tracks
        .iter()
        .find(|held| held["id"] == track)
        .expect("the track is there");
    let clip = &found["clips"][at];
    (
        clip["start"].as_u64().expect("a start"),
        clip["duration"].as_u64().expect("a duration"),
    )
}
