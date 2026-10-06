//! Making a generated asset's sketch with `asset_set`, and changing the rest
//! of its brief (#826): the free first draft a cut is laid out with, without
//! sending the whole document.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The asset row for `id`, as it is on disk.
fn asset(dir: &std::path::Path, id: &str) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    let document: Value = serde_json::from_str(&text).expect("the document is JSON");
    document["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .find(|row| row["id"] == json!(id))
        .cloned()
        .unwrap_or_else(|| panic!("no asset {id}"))
}

/// Each prompted kind is made in state `sketch`, carrying the block it was
/// given, and nothing is generated.
#[test]
fn each_prompted_kind_is_made_as_a_sketch() {
    let dir = project("sketch-kinds");
    for (kind, block, given) in [
        (
            "generated_video",
            "video",
            json!({ "seconds": 8, "aspect": "9:16" }),
        ),
        ("generated_image", "image", json!({ "resolution": "1K" })),
        (
            "generated_audio",
            "speech",
            json!({ "voice_id": "EXAMPLEvoiceID012345" }),
        ),
    ] {
        let (text, failed) = said(&call(
            "asset_set",
            json!({ "project": dir, "kind": kind, "asset": block,
                    "prompt": "a lone figure on a wet platform", block: given }),
        ));
        assert!(!failed, "{kind}: {text}");
        assert!(text.contains("nothing was generated"), "{text}");
        let row = asset(&dir, block);
        assert_eq!(row["kind"], json!(kind));
        assert_eq!(row["state"], json!("sketch"));
        for (field, value) in given.as_object().expect("an object") {
            assert_eq!(&row[block][field], value, "{kind}'s {field}");
        }
    }
    std::fs::remove_dir_all(dir).ok();
}

/// The block another kind takes is refused by name, and a sketch without its
/// sentence says what is missing.
#[test]
fn a_brief_of_the_wrong_kind_is_refused() {
    let dir = project("sketch-wrong");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "generated_video", "prompt": "a city",
                "speech": { "voice_id": "v" } }),
    ));
    assert!(failed, "a shot has no voice");
    assert!(text.contains("`speech`"), "got {text}");

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "generated_image", "image": { "model": "lite" } }),
    ));
    assert!(failed, "no prompt");
    assert!(text.contains("`prompt` is required"), "got {text}");

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "generated_audio", "prompt": "hello",
                "speech": { "voice": "v" } }),
    ));
    assert!(failed, "`voice` is not a field of the speech block");
    assert!(text.starts_with("`speech`:"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A voice is part of the brief: changing it on a generated line marks the
/// line stale exactly as rewording it would, and keeps the fields not named.
#[test]
fn changing_a_block_field_marks_a_generated_line_stale() {
    let dir = project("sketch-block");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "generated_audio", "asset": "line",
                "prompt": "hello", "speech": { "voice_id": "v1", "language": "en" } }),
    ));
    assert!(!failed, "{text}");
    // As `generate` would leave it: realised, with its file named.
    let path = dir.join("project.json");
    let mut document: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("JSON");
    for row in document["assets"].as_array_mut().expect("assets") {
        if row["id"] == json!("line") {
            row["state"] = json!("generated");
            row["path"] = json!("generated/line.mp3");
        }
    }
    std::fs::write(&path, document.to_string()).expect("write");

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "line", "speech": { "voice_id": "v2" } }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("stale"), "{text}");
    let row = asset(&dir, "line");
    assert_eq!(row["state"], json!("stale"));
    assert_eq!(row["speech"]["voice_id"], json!("v2"));
    assert_eq!(row["speech"]["language"], json!("en"), "the rest stayed");
    std::fs::remove_dir_all(dir).ok();
}
