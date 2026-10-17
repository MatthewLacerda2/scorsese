//! A ducked clip under a hand-written level (#1099): the dip `duck_music`
//! signed stays, whichever tool writes the volume afterwards, and multiplies
//! with it.

use super::fixture::project;
use crate::{call, said};
use scorsese_core::Project;
use serde_json::json;

/// Who wrote each of the music clip's volume tracks, in document order.
fn volume_authors(dir: &std::path::Path) -> Vec<Option<String>> {
    let project = Project::load(dir).expect("reload");
    project.tracks[1].clips[0]
        .keyframes
        .iter()
        .filter(|track| track.property.as_str() == "volume")
        .map(|track| track.by.clone())
        .collect()
}

fn ducked(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let (text, failed) = said(&call(
        "duck_music",
        json!({ "project": dir, "music": "music" }),
    ));
    assert!(!failed, "{text}");
    dir
}

#[test]
fn muting_a_ducked_clip_keeps_the_dip_beside_the_level() {
    let dir = ducked("volume-ducked");
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("muted"), "got {text}");
    assert!(
        !text.contains("replaced"),
        "nothing hand-written was there: {text}"
    );
    assert_eq!(volume_authors(&dir), [Some("duck".to_owned()), None]);

    // A second level replaces the first, and the dip still stays.
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.5 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("hand-written volume"), "got {text}");
    assert_eq!(volume_authors(&dir), [Some("duck".to_owned()), None]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn animating_volume_on_a_ducked_clip_leaves_the_dip_alone() {
    let dir = ducked("animate-ducked");
    let fade = json!([{ "at_seconds": 0.0, "value": 0.0 }, { "at_seconds": 1.0, "value": 1.0 }]);
    let (text, failed) = said(&call(
        "clip_animate",
        json!({ "project": dir, "clip": "m1", "property": "volume", "keyframes": fade }),
    ));
    assert!(!failed, "{text}");
    assert!(
        !text.contains("`duck` wrote"),
        "the dip is not replaced: {text}"
    );
    assert_eq!(volume_authors(&dir), [Some("duck".to_owned()), None]);

    let (text, failed) = said(&call(
        "clip_animate",
        json!({ "project": dir, "clip": "m1", "property": "volume", "keyframes": [] }),
    ));
    assert!(!failed, "{text}");
    assert_eq!(
        volume_authors(&dir),
        [Some("duck".to_owned())],
        "removing the hand's volume leaves the dip"
    );
    std::fs::remove_dir_all(dir).ok();
}
