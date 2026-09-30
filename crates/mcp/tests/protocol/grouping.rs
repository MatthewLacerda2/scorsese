//! Grouping clips into one layer and ungrouping them, over the wire.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The title becomes one clip of a new group, at the same place and time; and
/// ungrouping it puts the title back exactly where it was.
#[test]
fn a_clip_is_grouped_and_ungrouped_back_where_it_was() {
    let dir = project("group");
    let (text, failed) = said(&call(
        "clip_group",
        json!({ "project": dir, "clips": ["c1"], "asset": "titles" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`titles`"), "names the group: {text}");
    assert!(text.contains("clip `c-titles` on track `v1`"), "got {text}");

    let document = read(&dir);
    let group = asset(&document, "titles");
    assert_eq!(group["kind"], "group");
    assert_eq!(group["group"]["tracks"][0]["clips"][0]["id"], "c1");
    assert_eq!(document["tracks"][0]["clips"][0]["id"], "c-titles");

    let (text, failed) = said(&call(
        "clip_ungroup",
        json!({ "project": dir, "clip": "c-titles" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("1 clip(s) are back"), "got {text}");
    let document = read(&dir);
    assert!(
        document["assets"]
            .as_array()
            .expect("assets")
            .iter()
            .all(|asset| asset["id"] != "titles"),
        "the group is gone"
    );
    let back = &document["tracks"][1]["clips"][0];
    assert_eq!(
        (back["id"].clone(), back["start"].clone()),
        (json!("c1"), json!(0))
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A group is one picture, so sound is refused — and nothing is written.
#[test]
fn a_sound_is_not_grouped() {
    let dir = project("group-sound");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let (text, failed) = said(&call(
        "clip_group",
        json!({ "project": dir, "clips": ["m1"] }),
    ));
    assert!(failed, "the bed is sound");
    assert!(text.contains("picture only"), "got {text}");
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before
    );
    std::fs::remove_dir_all(dir).ok();
}

fn read(dir: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("read");
    serde_json::from_str(&text).expect("the server writes JSON")
}

fn asset<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .find(|asset| asset["id"] == id)
        .expect("the asset is there")
}
