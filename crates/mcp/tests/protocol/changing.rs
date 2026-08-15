//! The tools that change something, and what they refuse to change.

use super::fixture::{DOCUMENT, fingerprint, project};
use crate::{call, said};
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

#[test]
fn setting_a_level_writes_one_keyframe_and_says_what_the_clip_plays_at() {
    let dir = project("volume");
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.4 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("plays at 0.4"), "got {text}");
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(on_disk.contains(r#""property": "volume""#), "got {on_disk}");
    assert!(
        !on_disk.contains(r#""by""#),
        "a level is not signed: {on_disk}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The one interaction worth pinning: a level takes the volume lane over, so a
/// dip that was there is gone — and the reply has to say so rather than let it
/// be discovered on playback.
#[test]
fn muting_a_ducked_clip_replaces_the_dip_and_names_what_went() {
    let dir = project("volume-ducked");
    let (text, failed) = said(&call(
        "duck_music",
        json!({ "project": dir, "music": "music" }),
    ));
    assert!(!failed, "{text}");

    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("muted"), "got {text}");
    assert!(text.contains("`duck` wrote"), "got {text}");
    assert!(text.contains("duck_music"), "got {text}");
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(!on_disk.contains(r#""by": "duck""#), "got {on_disk}");
    std::fs::remove_dir_all(dir).ok();
}

/// A fade that finishes after the clip does is a fade nobody hears the end of,
/// so it is refused whole rather than quietly shortened.
#[test]
fn a_fade_that_runs_off_the_end_of_the_clip_changes_nothing() {
    let dir = project("volume-overrun");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let (text, failed) = said(&call(
        "set_volume",
        json!({ "project": dir, "clip": "m1", "level": 0.0,
                "from_level": 1.0, "at_seconds": 19.0, "seconds": 4.0 }),
    ));
    assert!(failed, "got {text}");
    assert!(text.contains("nothing was changed"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before
    );
    std::fs::remove_dir_all(dir).ok();
}

/// An agent writing `project.json` directly is the path that creates assets
/// nothing has probed, so the fix has to be reachable from here. This asserts
/// the half that needs no manufactured media: a file ffprobe cannot read is
/// named, and the document is left exactly as it was — a failed probe never
/// half-fills a `media` block.
#[test]
fn probing_names_the_file_it_could_not_read_and_writes_nothing() {
    let dir = project("probe");
    std::fs::write(dir.join("assets/shot.mp4"), b"not a video").expect("write the fake media");
    let with_footage = DOCUMENT.replace(
        r#"{ "id": "title", "kind": "text", "text": "TEASER" },"#,
        r#"{ "id": "title", "kind": "text", "text": "TEASER" },
    { "id": "shot", "kind": "video", "path": "assets/shot.mp4" },"#,
    );
    std::fs::write(dir.join("project.json"), &with_footage).expect("write the document");

    let (text, failed) = said(&call("project_probe", json!({ "project": dir })));

    assert!(!failed, "{text}");
    assert!(text.contains("shot"), "got {text}");
    assert!(text.contains("could not probe"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read back"),
        with_footage,
        "nothing was probed, so nothing was saved"
    );
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
