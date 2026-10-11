//! Calls side by side in one session (#1063): a read is not held up by a slow
//! call ahead of it, and edits to one project sent all at once each land.
//!
//! Every line is written before any answer is read, which is a client that
//! does not wait — Claude Code moving a slow call to the background, or
//! sending several calls in one turn.

use serde_json::{Value, json};

use super::fixture::project;
use crate::{exchange, said};

/// A `tools/call` frame as a line.
fn call(id: u64, name: &str, arguments: Value) -> String {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    })
    .to_string()
}

#[test]
fn a_read_answers_while_a_slow_call_ahead_of_it_is_still_running() {
    let dir = project("overlap-read");
    let render = json!({ "project": dir, "out": "cut.mp4", "resolution": "160x90", "wait": true });
    let lines = exchange(&[
        &call(1, "render", render),
        &call(2, "project_read", json!({ "project": dir })),
    ]);
    std::fs::remove_dir_all(&dir).ok();

    let ids: Vec<_> = lines.iter().map(|line| line["id"].clone()).collect();
    assert_eq!(ids, vec![json!(2), json!(1)], "the read waited: {lines:?}");
    let (text, failed) = said(&lines[1]);
    assert!(!failed && text.starts_with("wrote cut.mp4"), "{text}");
}

#[test]
fn edits_to_one_project_sent_at_once_all_land() {
    let dir = project("overlap-write");
    let lanes: Vec<String> = (1..=8).map(|n| format!("lane{n}")).collect();
    let frames: Vec<String> = lanes
        .iter()
        .zip(1..)
        .map(|(lane, id)| {
            call(
                id,
                "track_new",
                json!({ "project": dir, "kind": "video", "track": lane }),
            )
        })
        .collect();
    let lines = exchange(&frames.iter().map(String::as_str).collect::<Vec<_>>());
    let document = std::fs::read_to_string(dir.join("project.json")).expect("the project");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(lines.len(), lanes.len(), "{lines:?}");
    for line in &lines {
        let (text, failed) = said(line);
        assert!(!failed, "an edit was refused: {text}");
    }
    let saved: Value = serde_json::from_str(&document).expect("the project is JSON");
    let tracks: Vec<&str> = saved["tracks"]
        .as_array()
        .expect("tracks")
        .iter()
        .filter_map(|track| track["id"].as_str())
        .collect();
    for lane in &lanes {
        assert!(
            tracks.contains(&lane.as_str()),
            "{lane} was lost: {tracks:?}"
        );
    }
}
