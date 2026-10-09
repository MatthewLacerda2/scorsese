//! Sound effects that overlap, placed without picking a free track (#972).

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// `music` is taken for its whole length, so a sound asked onto it with
/// `spill_over` goes to a new `music-2`, and the reply says so.
#[test]
fn an_overlapping_sound_lands_on_the_next_lane_and_the_reply_names_it() {
    let dir = project("spill");
    for expected in ["music-2", "music-3"] {
        let (text, failed) = said(&call(
            "place_clip",
            json!({ "project": dir, "asset": "vo", "track": "music",
                    "start_seconds": 1.0, "duration_seconds": 1.0, "spill_over": true }),
        ));
        assert!(!failed, "{text}");
        assert!(text.contains(&format!("on `{expected}`")), "got {text}");
    }
    let project: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).expect("read"))
            .expect("JSON");
    let ids: Vec<_> = project["tracks"]
        .as_array()
        .expect("tracks")
        .iter()
        .map(|track| track["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(ids, ["v1", "music", "music-2", "music-3", "vo"]);
    std::fs::remove_dir_all(dir).ok();
}

/// A picture's track is what it is drawn over, so it never spills.
#[test]
fn a_video_track_does_not_spill() {
    let dir = project("spill-video");
    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "title", "track": "v1",
                "start_seconds": 1.0, "duration_seconds": 1.0, "spill_over": true }),
    ));
    assert!(failed, "v1 is a video track");
    assert!(text.contains("not an audio track"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
