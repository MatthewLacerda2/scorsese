//! Where a video lane sits in the stack: added under another, or moved (#1117).

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The track ids, bottom to top.
fn order(dir: &std::path::Path) -> Vec<String> {
    let written = std::fs::read_to_string(dir.join("project.json")).expect("on disk");
    let document: Value = serde_json::from_str(&written).expect("json");
    document["tracks"]
        .as_array()
        .expect("tracks")
        .iter()
        .map(|track| track["id"].as_str().expect("an id").to_owned())
        .collect()
}

#[test]
fn a_new_video_track_can_go_under_another_and_then_be_moved() {
    let dir = project("layering");
    let before = order(&dir);
    let bottom = before.first().expect("the fixture has a lane").clone();
    let (text, failed) = said(&call(
        "track_new",
        json!({ "project": dir, "kind": "video", "track": "logo", "under": bottom }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains(&format!("directly under `{bottom}`")),
        "{text}"
    );
    assert_eq!(order(&dir)[0], "logo");

    let (text, failed) = said(&call(
        "track_move",
        json!({ "project": dir, "track": "logo", "over": bottom }),
    ));
    assert!(!failed, "{text}");
    assert_eq!(text, format!("`logo` is now directly over `{bottom}`."));
    assert_eq!(order(&dir)[..2], [bottom.clone(), "logo".to_owned()]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_move_with_nowhere_or_two_places_to_go_writes_nothing() {
    let dir = project("layering-refused");
    let before = order(&dir);
    let track = before.first().expect("a lane").clone();
    for (arguments, says) in [
        (
            json!({ "project": dir, "track": track }),
            "`under` or `over` is required",
        ),
        (
            json!({ "project": dir, "track": track, "under": "x", "over": "y" }),
            "not both",
        ),
        (
            json!({ "project": dir, "track": track, "under": "nowhere" }),
            "no track in this project is called `nowhere`",
        ),
    ] {
        let (text, failed) = said(&call("track_move", arguments));
        assert!(failed && text.contains(says), "{text}");
    }
    assert_eq!(order(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}
