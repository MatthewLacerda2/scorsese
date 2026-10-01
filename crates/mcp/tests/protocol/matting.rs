//! `clip_set`'s `matte`: set, turned inside out, removed, and refused where
//! the document would not load.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The fixture with a second picture clip, `c-wipe`, over the title.
fn with_wipe(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let path = dir.join("project.json");
    let mut document: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("JSON");
    document["tracks"]
        .as_array_mut()
        .expect("tracks")
        .push(json!({ "id": "v2", "kind": "video", "clips": [
            { "id": "c-wipe", "asset": "title", "start": 0, "duration": 600 } ] }));
    std::fs::write(&path, document.to_string()).expect("write");
    dir
}

fn matte_of(dir: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let project: Value = serde_json::from_str(&text).expect("the server writes JSON");
    project["tracks"][0]["clips"][0]["matte"].clone()
}

fn set(dir: &std::path::Path, matte: Value) -> (String, bool) {
    said(&call(
        "clip_set",
        json!({ "project": dir, "clip": "c1", "matte": matte }),
    ))
}

#[test]
fn a_matte_is_set_inverted_and_removed() {
    let dir = with_wipe("clip-set-matte");
    let (text, failed) = set(&dir, json!({ "clip": "c-wipe" }));
    assert!(!failed && text.contains("through clip `c-wipe`"), "{text}");
    assert_eq!(matte_of(&dir), json!({ "clip": "c-wipe" }));

    let (text, failed) = set(&dir, json!({ "invert": true }));
    assert!(!failed && text.contains("outside clip `c-wipe`"), "{text}");
    assert_eq!(matte_of(&dir), json!({ "clip": "c-wipe", "invert": true }));

    let (text, failed) = set(&dir, json!(false));
    assert!(!failed && text.contains("no matte"), "{text}");
    assert_eq!(matte_of(&dir), Value::Null);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_matte_the_document_would_refuse_writes_nothing() {
    let dir = with_wipe("clip-set-matte-refused");
    for (matte, expected) in [
        (json!({ "clip": "m1" }), "audio track"),
        (json!({ "clip": "c1" }), "masked by itself"),
        (json!({ "clip": "nowhere" }), "does not exist"),
        (json!({ "luma": true }), "not `luma`"),
        (json!({ "invert": true }), "needs a `clip`"),
        (json!(true), "or `false`"),
    ] {
        let (text, failed) = set(&dir, matte);
        assert!(failed && text.contains(expected), "{expected:?} in {text}");
    }
    assert_eq!(matte_of(&dir), Value::Null, "nothing was written");
    std::fs::remove_dir_all(dir).ok();
}
