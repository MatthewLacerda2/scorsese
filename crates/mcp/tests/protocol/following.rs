//! `clip_follow`: a clip sent along an arrow over the wire, and brought back.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

fn clip_of(dir: &std::path::Path, id: &str) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk");
    let document: Value = serde_json::from_str(&text).expect("the project is JSON");
    document["tracks"]
        .as_array()
        .expect("tracks")
        .iter()
        .flat_map(|track| track["clips"].as_array().cloned().unwrap_or_default())
        .find(|clip| clip["id"] == id)
        .expect("the clip is on the timeline")
}

/// The title's project, with an arrow placed on a track of its own as `c-route`.
fn with_a_route(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    for (tool, arguments) in [
        (
            "asset_set",
            json!({ "project": dir, "kind": "shape", "geometry": "arrow", "stroke": "#ffffff", "asset": "route",
                    "from": { "x": 0.1, "y": 0.5 }, "to": { "x": 0.9, "y": 0.5 }, "curve": "s" }),
        ),
        (
            "track_new",
            json!({ "project": dir, "kind": "video", "id": "v2" }),
        ),
        (
            "place_clip",
            json!({ "project": dir, "asset": "route", "track": "v2", "start_seconds": 0,
                    "duration_seconds": 20, "clip": "c-route" }),
        ),
    ] {
        let (text, failed) = said(&call(tool, arguments));
        assert!(!failed, "{tool}: {text}");
    }
    dir
}

/// One call writes both halves: the follow, and a ramp from tail to head.
#[test]
fn a_clip_is_sent_along_an_arrow_with_its_ramp() {
    let dir = with_a_route("follow-set");
    let (text, failed) = said(&call(
        "clip_follow",
        json!({ "project": dir, "clip": "c1", "arrow": "c-route", "orient": true,
                "start_seconds": 1, "travel_seconds": 2, "easing": "ease_in_out" }),
    ));
    assert!(!failed, "{text}");
    let c1 = clip_of(&dir, "c1");
    assert_eq!(c1["follow"], json!({ "clip": "c-route", "orient": true }));
    let progress = &c1["keyframes"][0];
    assert_eq!(progress["property"], "follow.progress");
    let keys = progress["keyframes"].as_array().expect("keyframes");
    let read: Vec<(u64, f64)> = keys
        .iter()
        .map(|key| {
            (
                key["t"].as_u64().expect("t"),
                key["value"].as_f64().expect("v"),
            )
        })
        .collect();
    assert_eq!(read, [(30, 0.0), (90, 1.0)]);
    assert_eq!(keys[0]["easing"], "ease_in_out");

    let (text, failed) = said(&call(
        "clip_follow",
        json!({ "project": dir, "clip": "c1", "stop": true }),
    ));
    assert!(!failed, "{text}");
    let c1 = clip_of(&dir, "c1");
    assert!(
        c1.get("follow").is_none() && c1.get("keyframes").is_none(),
        "{c1}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A follow naming something that is not an arrow is refused in validation's
/// words, and nothing is written.
#[test]
fn following_what_is_not_an_arrow_writes_nothing() {
    let dir = with_a_route("follow-refused");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let (text, failed) = said(&call(
        "clip_follow",
        json!({ "project": dir, "clip": "c-route", "arrow": "c1" }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("only a clip of an arrow"), "{text}");
    let after = std::fs::read_to_string(dir.join("project.json")).expect("read");
    assert_eq!(before, after);
    std::fs::remove_dir_all(dir).ok();
}
