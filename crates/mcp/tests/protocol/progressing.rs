//! A render followed as it runs (#700): a waited call reports progress when it
//! carries a `progressToken`, a background one answers at once, and a session
//! that ends takes its renders with it.

use serde_json::{Value, json};

use super::fixture::project;
use crate::{exchange, said};

/// A `tools/call` frame for `render` with `id`, as a line.
fn render(id: u64, arguments: Value, meta: Option<Value>) -> String {
    let mut params = json!({ "name": "render", "arguments": arguments });
    if let Some(meta) = meta {
        params["_meta"] = meta;
    }
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }).to_string()
}

#[test]
fn a_waited_render_with_a_token_reports_rising_progress_before_its_answer() {
    let dir = project("progress-waited");
    let arguments = json!({
        "project": dir, "out": "cut.mp4", "resolution": "160x90",
        "range": "0:30", "wait": true
    });
    let lines = exchange(&[&render(
        4,
        arguments,
        Some(json!({ "progressToken": "bar" })),
    )]);
    std::fs::remove_dir_all(&dir).ok();

    let (reply, notes) = lines.split_last().expect("something was written");
    let (text, failed) = said(reply);
    assert!(!failed && text.starts_with("wrote cut.mp4"), "{text}");
    assert!(!notes.is_empty(), "no progress was reported: {lines:?}");
    let mut last = -1.0;
    for note in notes {
        assert_eq!(note["method"], "notifications/progress", "{note}");
        assert!(note.get("id").is_none(), "a notification has no id: {note}");
        let params = &note["params"];
        assert_eq!(params["progressToken"], "bar");
        assert_eq!(params["total"], 100);
        assert!(params["message"].as_str().is_some_and(|m| !m.is_empty()));
        let progress = params["progress"].as_f64().expect("a number");
        assert!(
            progress > last && progress < 100.0,
            "{progress} after {last}"
        );
        last = progress;
    }
}

#[test]
fn a_waited_render_without_a_token_sends_nothing_but_its_answer() {
    let dir = project("progress-quiet");
    let arguments = json!({
        "project": dir, "out": "cut.mp4", "resolution": "160x90",
        "range": "0:3", "wait": true
    });
    let lines = exchange(&[&render(4, arguments, None)]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(said(&lines[0]).0.starts_with("wrote cut.mp4"));
}

/// The fixture's whole twenty seconds: far longer than the moment between the
/// answer and the end of the input, so the render is still running when the
/// session ends.
#[test]
fn a_background_render_answers_at_once_and_stops_with_the_session() {
    let dir = project("progress-ended");
    let arguments = json!({ "project": dir, "out": "cut.mp4", "resolution": "160x90" });
    let again = json!({ "project": dir, "out": "cut.mp4", "resolution": "160x90" });
    let lines = exchange(&[&render(1, arguments, None), &render(2, again, None)]);
    let left = dir.join("cut.mp4").exists();
    std::fs::remove_dir_all(&dir).ok();

    let (text, failed) = said(&lines[0]);
    assert!(!failed, "{text}");
    assert!(text.starts_with("Rendering cut.mp4 as job 1"), "{text}");
    assert!(text.contains("jobs with job: 1"), "{text}");
    let (text, failed) = said(&lines[1]);
    assert!(
        failed,
        "a second render of the same file is refused: {text}"
    );
    assert!(text.starts_with("job 1 is still writing cut.mp4"), "{text}");
    assert!(!left, "the render the session left behind was not removed");
}
