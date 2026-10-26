//! The tools that change something, and what they refuse to change.

use super::fixture::{DOCUMENT, document, fingerprint, project};
use crate::protocol::{call, said};
use serde_json::json;

/// The whole edit is the document, so writing it is how any change is made:
/// read it, change it, write it back with the fingerprint that read reported.
#[test]
fn writing_a_document_replaces_it() {
    let dir = project("write");
    let renamed = DOCUMENT.replace("Teaser", "Trailer");
    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": renamed, "fingerprint": fingerprint(&dir) }),
    ));
    assert!(!failed, "{text}");
    let on_disk = document(&dir);
    assert!(on_disk.contains("Trailer"));
    std::fs::remove_dir_all(dir).ok();
}

/// The property that makes `project_write` safe to hand an assistant: a
/// document that would not load never reaches the disk.
#[test]
fn writing_a_broken_document_changes_nothing() {
    let dir = project("write-broken");
    let before = document(&dir);
    let broken = DOCUMENT.replace(r#""asset": "bed""#, r#""asset": "ghost""#);

    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": broken }),
    ));
    assert!(failed, "a broken document must be refused");
    assert!(text.contains("nothing written"), "got {text}");
    assert_eq!(document(&dir), before, "the file on disk must be untouched");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn setting_a_level_writes_one_keyframe_and_says_what_the_clip_plays_at() {
    let dir = project("volume");
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.4 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("plays at 0.4"), "got {text}");
    let on_disk = document(&dir);
    assert!(on_disk.contains(r#""property": "volume""#), "got {on_disk}");
    assert!(
        !on_disk.contains(r#""by""#),
        "a level is not signed: {on_disk}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A fade that finishes after the clip does is a fade nobody hears the end of,
/// so it is refused whole rather than quietly shortened.
#[test]
fn a_fade_that_runs_off_the_end_of_the_clip_changes_nothing() {
    let dir = project("volume-overrun");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.0,
                "from_level": 1.0, "at_seconds": 19.0, "seconds": 4.0 }),
    ));
    assert!(failed, "got {text}");
    assert!(text.contains("nothing was changed"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// A tool that refuses is a *successful* call whose answer is a refusal —
/// that is the distinction MCP draws, and it decides whether a client shows
/// the message or reports a broken connection.
#[test]
fn a_refusal_is_a_result_with_is_error_rather_than_a_protocol_error() {
    let reply = call("project_read", json!({ "project": "/no/such/place.scor" }));
    assert!(reply.get("error").is_none(), "got {reply}");
    let (text, failed) = said(&reply);
    assert!(failed && !text.is_empty());
}

#[test]
fn a_missing_project_argument_is_refused_by_name() {
    let (text, failed) = said(&call("project_describe", json!({})));
    assert!(failed && text.contains("project"), "got {text}");
}

#[test]
fn calling_a_tool_that_does_not_exist_is_a_protocol_error() {
    let reply = call("make_it_good", json!({}));
    assert_eq!(reply["error"]["code"], json!(-32601));
}
