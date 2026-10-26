//! A folder of frames in as one image sequence, then retimed and made anew
//! from the stills it brought.

use std::path::PathBuf;

use super::fixture::{document, project};

use crate::protocol::{call, said};
use scorsese_render::testing::{still, tools};
use serde_json::json;

/// A folder of three numbered frames, named so a plain sort plays them wrong.
fn frames(label: &str) -> PathBuf {
    let dir = crate::protocol::scratch::fresh(&format!("scorsese-mcp-{label}"), "").join("spin");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create the frame folder");
    for (name, colour) in [("f_1", "red"), ("f_2", "lime"), ("f_10", "blue")] {
        still(&tools(), &dir.join(format!("{name}.png")), colour, "64x64");
    }
    dir
}

#[test]
fn a_folder_of_frames_becomes_one_sequence_that_can_be_retimed() {
    let dir = project("sequence");
    let outside = frames("sequence-frames");

    let reply = call(
        "import",
        json!({ "project": dir, "path": outside, "sequence": true }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(
        text.contains("spin — image_sequence: 3 stills × 1 frame(s) = 3 frames"),
        "got {text}"
    );
    assert!(text.contains("stills spin-f_1 to spin-f_10"), "got {text}");
    assert!(
        text.contains("gap: 7 missing between f_2.png and f_10.png"),
        "got {text}"
    );

    let reply = call(
        "sequence",
        json!({ "project": dir, "asset": "spin", "hold": 4, "loop": true }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(
        text.contains("now 3 stills × 4 frame(s) = 12 frames, looping"),
        "got {text}"
    );

    let reply = call(
        "sequence",
        json!({ "project": dir, "asset": "blink", "stills": ["spin-f_2", "spin-f_1"] }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(text.contains("blink — made: 2 stills"), "got {text}");

    let reply = call(
        "sequence",
        json!({ "project": dir, "asset": "spin", "hold": 0 }),
    );
    let (text, failed) = said(&reply);
    assert!(failed, "a hold of nothing is refused: {text}");

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside.parent().expect("a parent")).ok();
}

/// A clip showing a sequence animates like any other: `clip_animate` checks
/// the property against the published table, never the asset's kind.
#[test]
fn a_clip_of_a_sequence_animates_like_any_other() {
    let dir = project("sequence-animate");
    let outside = frames("sequence-animate-frames");
    let import = json!({ "project": dir, "path": outside, "sequence": true });
    assert!(!said(&call("import", import)).1);
    let place = json!({ "project": dir, "asset": "spin", "track": "v1",
                        "start_seconds": 25.0, "duration_seconds": 1.0 });
    let (text, failed) = said(&call("place_clip", place));
    assert!(!failed, "{text}");

    let read = || -> serde_json::Value {
        let text = document(&dir);
        serde_json::from_str(&text).expect("JSON")
    };
    let clips = read()["tracks"][0]["clips"].clone();
    let clip = clips
        .as_array()
        .expect("clips")
        .iter()
        .find(|c| c["asset"] == "spin");
    let id = clip.expect("the sequence's clip")["id"].clone();

    let pop = json!([{ "at_seconds": 0, "value": 0.6 }, { "at_seconds": 0.5, "value": 1 }]);
    let animate = json!({ "project": dir, "clip": id, "property": "transform.scale",
                          "keyframes": pop });
    let (text, failed) = said(&call("clip_animate", animate));
    assert!(!failed, "{text}");
    let clips = read()["tracks"][0]["clips"].clone();
    let clip = clips
        .as_array()
        .expect("clips")
        .iter()
        .find(|c| c["id"] == id);
    let tracks = clip.expect("still there")["keyframes"].clone();
    let paths: Vec<_> = tracks
        .as_array()
        .expect("tracks")
        .iter()
        .map(|t| t["property"].clone())
        .collect();
    assert_eq!(
        paths,
        [json!("transform.scale.x"), json!("transform.scale.y")]
    );

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside.parent().expect("a parent")).ok();
}
