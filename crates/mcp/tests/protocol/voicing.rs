//! `cut_to_voice` laying a cut out from its lines (#1008).

use serde_json::{Value, json};

use super::fixture::project;
use crate::{call, said};

/// Where clip `id` sits in the saved document: its start and duration.
fn placed(dir: &std::path::Path, id: &str) -> (u64, u64) {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is saved");
    let document: Value = serde_json::from_str(&text).expect("the project is JSON");
    let clip = document["tracks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|track| track["clips"].as_array())
        .flatten()
        .find(|clip| clip["id"] == id)
        .expect("the clip is still in the project");
    let frames = |key: &str| clip[key].as_u64().expect("a whole number of frames");
    (frames("start"), frames("duration"))
}

#[test]
fn an_untimed_line_ends_its_scene_the_gap_after_its_audio() {
    let dir = project("voicing");
    let scenes = json!([{ "line": "v1c", "visuals": ["c1"] }]);
    let (text, failed) = said(&call(
        "cut_to_voice",
        json!({ "project": dir, "scenes": scenes }),
    ));
    assert!(!failed, "{text}");
    // The line moves to the scene's start, frame 0, and runs its 90 frames;
    // 0.2 s more is 96 frames, 3.20 s, where the title now ends.
    assert!(
        text.contains("the cut now ends at 3.20s (was 20.00s)"),
        "{text}"
    );
    assert!(text.contains("no word timings"), "{text}");
    assert!(text.contains("bake it again"), "the bed is a score: {text}");
    assert_eq!(placed(&dir, "v1c"), (0, 90));
    assert_eq!(placed(&dir, "c1"), (0, 96));
    assert_eq!(placed(&dir, "m1"), (0, 600), "named nowhere, so left alone");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_refused_cut_changes_nothing() {
    let dir = project("voicing-refused");
    let before = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let scenes = json!([{ "line": "v1c", "visuals": ["c1"], "moves_with": ["nope"] }]);
    let (text, failed) = said(&call(
        "cut_to_voice",
        json!({ "project": dir, "scenes": scenes }),
    ));
    assert!(failed, "{text}");
    assert!(
        text.contains("no clip `nope`") && text.contains("nothing was changed"),
        "{text}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).unwrap(),
        before
    );
    std::fs::remove_dir_all(dir).ok();
}
