//! `clip_animate`: one property's keyframes written, replaced and removed over
//! the wire, without touching any other.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

fn document(dir: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk");
    serde_json::from_str(&text).expect("the project is JSON")
}

/// The title's keyframe tracks, as `(property, [(t, value)])`.
fn tracks_of(clip: &Value) -> Vec<(String, Vec<(u64, f64)>)> {
    let tracks = clip["keyframes"].as_array().cloned().unwrap_or_default();
    tracks
        .iter()
        .map(|track| {
            let points = track["keyframes"].as_array().expect("points");
            let read = points.iter().map(|key| {
                (
                    key["t"].as_u64().expect("t"),
                    key["value"].as_f64().expect("value"),
                )
            });
            (
                track["property"].as_str().expect("property").to_owned(),
                read.collect(),
            )
        })
        .collect()
}

fn animate(dir: &std::path::Path, clip: &str, property: &str, keyframes: Value) -> (String, bool) {
    said(&call(
        "clip_animate",
        json!({ "project": dir, "clip": clip, "property": property, "keyframes": keyframes }),
    ))
}

/// A pop: `transform.scale` writes both axes, rounded onto the grid with its
/// easing kept, and a fade already on the clip is left exactly as it was.
#[test]
fn a_scale_pop_writes_both_axes_and_leaves_the_fade_alone() {
    let dir = project("animate-pop");
    let fade = json!([{ "at_seconds": 0, "value": 0 }, { "at_seconds": 0.5, "value": 1 }]);
    let (text, failed) = animate(&dir, "c1", "opacity", fade);
    assert!(!failed, "{text}");

    let pop = json!([{ "at_seconds": 0.45, "value": 1.0 },
                     { "at_seconds": 0, "value": 0.6, "easing": "back_out" }]);
    let (text, failed) = animate(&dir, "c1", "transform.scale", pop);
    assert!(!failed, "{text}");
    assert!(text.contains("0.47s (frame 14)"), "both units: {text}");
    assert!(text.contains("back_out"), "names the easing: {text}");

    let c1 = &document(&dir)["tracks"][0]["clips"][0];
    let points = vec![(0, 0.6), (14, 1.0)];
    assert_eq!(
        tracks_of(c1),
        [
            ("opacity".to_owned(), vec![(0, 0.0), (15, 1.0)]),
            ("transform.scale.x".to_owned(), points.clone()),
            ("transform.scale.y".to_owned(), points),
        ]
    );
    assert_eq!(c1["keyframes"][1]["keyframes"][0]["easing"], "back_out");
    assert!(c1["keyframes"][1].get("by").is_none(), "unsigned: {c1}");
    std::fs::remove_dir_all(dir).ok();
}

/// A second call replaces the property's track and says what went; an empty
/// list removes it.
#[test]
fn a_track_is_replaced_and_then_removed() {
    let dir = project("animate-replace");
    let once = json!([{ "at_seconds": 0, "value": 0 }, { "at_seconds": 1, "value": 1 }]);
    assert!(!animate(&dir, "c1", "glow.intensity", once).1);
    let again = json!([{ "at_seconds": 2, "value": 3, "easing": { "cubic_bezier": [0.25, 0.1, 0.25, 1] } }]);
    let (text, failed) = animate(&dir, "c1", "glow.intensity", again);
    assert!(!failed, "{text}");
    assert!(text.contains("2 point(s) written by hand"), "got {text}");
    let c1 = &document(&dir)["tracks"][0]["clips"][0];
    assert_eq!(
        tracks_of(c1),
        [("glow.intensity".to_owned(), vec![(60, 3.0)])]
    );

    let (text, failed) = animate(&dir, "c1", "glow.intensity", json!([]));
    assert!(!failed, "{text}");
    assert!(text.contains("no longer animated"), "got {text}");
    assert!(
        document(&dir)["tracks"][0]["clips"][0]
            .get("keyframes")
            .is_none()
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A misspelt property, a point past the clip's end, and two points on one
/// frame are each refused with the reason, and nothing is written.
#[test]
fn what_cannot_be_animated_writes_nothing() {
    let dir = project("animate-refused");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let point = json!([{ "at_seconds": 0, "value": 1 }]);
    let (text, failed) = animate(&dir, "c1", "opactiy", point);
    assert!(failed && text.contains("did you mean `opacity`?"), "{text}");

    let late = json!([{ "at_seconds": 30, "value": 1 }]);
    let (text, failed) = animate(&dir, "c1", "opacity", late);
    assert!(failed && text.contains("only 600 frames long"), "{text}");

    let same = json!([{ "at_seconds": 1, "value": 0 }, { "at_seconds": 1.01, "value": 1 }]);
    let (text, failed) = animate(&dir, "c1", "opacity", same);
    assert!(failed && text.contains("both land on frame 30"), "{text}");

    let after = std::fs::read_to_string(dir.join("project.json")).expect("read");
    assert_eq!(before, after);
    std::fs::remove_dir_all(dir).ok();
}

/// A member of a group is found by its id, inside the group's own tracks.
#[test]
fn a_member_of_a_group_is_animated_in_place() {
    let dir = project("animate-member");
    let (text, failed) = said(&call(
        "clip_group",
        json!({ "project": dir, "clips": ["c1"], "asset": "titles" }),
    ));
    assert!(!failed, "{text}");
    let (text, failed) = animate(
        &dir,
        "c1",
        "transform.rotation",
        json!([{ "at_seconds": 0, "value": 15 }]),
    );
    assert!(!failed, "{text}");
    let document = document(&dir);
    let group = document["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .find(|asset| asset["id"] == "titles")
        .expect("the group");
    let member = &group["group"]["tracks"][0]["clips"][0];
    assert_eq!(
        tracks_of(member),
        [("transform.rotation".to_owned(), vec![(0, 15.0)])]
    );
    std::fs::remove_dir_all(dir).ok();
}
