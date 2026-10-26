//! Scaling a run of clips about an instant, over the wire.

use super::fixture::{document, project, read};
use crate::protocol::{call, said};
use serde_json::{Value, json};

/// The plain case: the clips move, their durations move with them because a
/// title's length is the timeline's to decide, and the reply says what it did.
#[test]
fn scaling_the_pacing_moves_the_clips_and_stretches_what_it_may() {
    let dir = project("pace");
    let (text, failed) = said(&call(
        "scale_pacing",
        json!({ "project": dir, "clips": ["c1", "v1c"], "factor": 0.5 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("2 clips scaled ×0.5"), "got {text}");

    let after = read(&dir);
    assert_eq!(shape(&after, "v1", 0), (0, 300), "the title, halved");
    assert_eq!(shape(&after, "vo", 0), (30, 45), "the line, drawn in");
    std::fs::remove_dir_all(dir).ok();
}

/// A title's fade scales with it, and the reply says so; a tool's ramp is
/// left alone and its tool named, so the caller knows what to run again.
#[test]
fn keyframes_scale_with_the_clip_and_tool_tracks_are_named() {
    let dir = project("pace-keys");
    let mut doc = read(&dir);
    doc["tracks"][0]["clips"][0]["keyframes"] = json!([
        { "property": "opacity",
          "keyframes": [ { "t": 0, "value": 1.0 }, { "t": 600, "value": 0.0 } ] },
        { "property": "transform.scale", "by": "dissolve",
          "keyframes": [ { "t": 0, "value": 1.0 }, { "t": 30, "value": 1.2 } ] }
    ]);
    std::fs::write(dir.join("project.json"), doc.to_string()).expect("write");

    let (text, failed) = said(&call(
        "scale_pacing",
        json!({ "project": dir, "clips": ["c1"], "factor": 0.5 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("Keyframes on `c1` were scaled"), "got {text}");
    assert!(text.contains("run `dissolve` again"), "got {text}");

    let keys = &read(&dir)["tracks"][0]["clips"][0]["keyframes"];
    assert_eq!(
        keys[0]["keyframes"][1]["t"], 300,
        "the fade ends at the new end"
    );
    assert_eq!(
        keys[1]["keyframes"][1]["t"], 30,
        "the tool's ramp is untouched"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The promise every refusal here carries: the document on disk is untouched,
/// so a client may keep asking until it asks for something possible.
#[test]
fn a_factor_that_is_not_a_scale_changes_nothing() {
    let dir = project("pace-refused");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "scale_pacing",
        json!({ "project": dir, "clips": ["c1"], "factor": 0.0 }),
    ));
    assert!(failed, "×0 collapses the cut onto a point");
    assert!(text.contains("nothing was changed"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
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
