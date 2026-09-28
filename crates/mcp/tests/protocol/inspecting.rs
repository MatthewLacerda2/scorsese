//! `clip_set`: the values an inspector shows, set over the wire.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

fn clip(dir: &std::path::Path, track: usize) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let project: Value = serde_json::from_str(&text).expect("the server writes JSON");
    project["tracks"][track]["clips"][0].clone()
}

/// Scale is both axes, each one held point — and setting it again replaces
/// that point rather than stacking a second track on the property.
#[test]
fn a_transform_is_one_held_point_per_axis_and_replaces_itself() {
    let dir = project("clip-set-scale");
    for size in [0.5, 0.8] {
        let (text, failed) = said(&call(
            "clip_set",
            json!({ "project": dir, "clip": "c1", "scale": size, "rotation": 15 }),
        ));
        assert!(!failed, "{text}");
        assert!(text.contains("Nothing else changed"), "got {text}");
    }
    let keyframes = clip(&dir, 0)["keyframes"].clone();
    let tracks = keyframes.as_array().expect("keyframes were written");
    assert_eq!(
        tracks.len(),
        3,
        "scale.x, scale.y and rotation: {keyframes}"
    );
    let scale = tracks
        .iter()
        .find(|track| track["property"] == "transform.scale.x")
        .expect("scale.x");
    assert_eq!(scale["keyframes"].as_array().map(Vec::len), Some(1));
    assert_eq!(scale["keyframes"][0]["value"], 0.8);
    std::fs::remove_dir_all(dir).ok();
}

/// Speed retimes: the same footage in half the time, as a 2× button means.
#[test]
fn a_speed_shortens_the_clip_to_the_same_footage() {
    let dir = project("clip-set-speed");
    let (text, failed) = said(&call(
        "clip_set",
        json!({ "project": dir, "clip": "c1", "speed": 2.0, "fit": "fill" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("10.00s"), "the new length is said: {text}");
    let c1 = clip(&dir, 0);
    assert_eq!(
        (c1["duration"].as_u64(), c1["fit"].as_str()),
        (Some(300), Some("fill"))
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A picture value on a sound, a size of nothing and an empty call are each
/// refused by name, and the document is left exactly as it was.
#[test]
fn what_cannot_apply_is_refused_and_nothing_is_written() {
    let dir = project("clip-set-refused");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    for (mut arguments, expected) in [
        (json!({ "clip": "m1", "rotation": 90 }), "audio track"),
        (json!({ "clip": "c1", "scale": 0 }), "not a size"),
        (json!({ "clip": "c1", "speed": -1 }), "positive"),
        (json!({ "clip": "c1" }), "nothing to set"),
        (json!({ "clip": "nope", "scale": 2 }), "no clip `nope`"),
    ] {
        arguments["project"] = json!(dir);
        let (text, failed) = said(&call("clip_set", arguments));
        assert!(failed, "{text}");
        assert!(text.contains(expected), "expected {expected:?} in {text}");
    }
    let after = std::fs::read_to_string(dir.join("project.json")).expect("read");
    assert_eq!(after, before);
    std::fs::remove_dir_all(dir).ok();
}
