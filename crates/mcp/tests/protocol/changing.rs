//! The tools that change something, and what they refuse to change.

use super::fixture::{DOCUMENT, project};
use crate::{call, said};
use serde_json::json;

/// The whole edit is the document, so writing it is how any change is made.
#[test]
fn writing_a_document_replaces_it() {
    let dir = project("write");
    let renamed = DOCUMENT.replace("Teaser", "Trailer");
    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": renamed }),
    ));
    assert!(!failed, "{text}");
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(on_disk.contains("Trailer"));
    std::fs::remove_dir_all(dir).ok();
}

/// The property that makes `project_write` safe to hand an assistant: a
/// document that would not load never reaches the disk.
#[test]
fn writing_a_broken_document_changes_nothing() {
    let dir = project("write-broken");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let broken = DOCUMENT.replace(r#""asset": "bed""#, r#""asset": "ghost""#);

    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": broken }),
    ));
    assert!(failed, "a broken document must be refused");
    assert!(text.contains("nothing written"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before,
        "the file on disk must be untouched"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn ducking_writes_keyframes_and_says_how_many() {
    let dir = project("duck");
    let (text, failed) = said(&call(
        "duck_music",
        json!({ "project": dir, "music": "music" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("1 clip(s) ducked"), "got {text}");
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(on_disk.contains(r#""by": "duck""#), "got {on_disk}");
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
