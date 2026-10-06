//! `hear` answers with the numbers too (#782): the waveform's sentence, then a
//! line of levels, with the section rows and a comparison only when asked for.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

/// A song's opening baked twice — every track, and its lead alone — so there
/// are two files that differ. Partial bakes, because only a partial bake
/// lands at the `out` it is given.
fn two_bakes(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );
    ok(
        "synth_bake",
        json!({ "project": dir, "asset": "theme", "beats": "0:32",
                "out": "cache/whole.wav" }),
    );
    ok(
        "synth_bake",
        json!({ "project": dir, "asset": "theme", "beats": "0:32",
                "only": ["lead"], "out": "cache/lead.wav" }),
    );
    dir
}

/// The default reply is the picture's sentence and one summary line: no
/// section rows, no comparison — the lean reply #779 measured as the one to
/// keep.
#[test]
fn the_default_reply_is_the_picture_and_one_line_of_levels() {
    let dir = two_bakes("hear-lean");
    let text = ok("hear", json!({ "project": dir, "file": "cache/whole.wav" }));
    let levels: Vec<&str> = text.lines().filter(|l| l.starts_with("levels")).collect();
    assert_eq!(levels.len(), 1, "one summary line: {text}");
    assert!(levels[0].contains("mean"), "a mean level: {text}");
    assert_eq!(text.lines().count(), 2, "nothing else by default: {text}");
    assert!(!text.contains(" vs "), "no comparison unasked: {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// `sections` adds a row per section; `against` compares the two files by
/// name, field by field.
#[test]
fn sections_and_against_add_rows_and_a_comparison() {
    let dir = two_bakes("hear-full");
    let text = ok(
        "hear",
        json!({ "project": dir, "file": "cache/whole.wav",
                "against": "cache/lead.wav", "sections": true }),
    );
    assert!(
        text.lines().filter(|l| l.starts_with("  ")).count() > 1,
        "indented rows follow the summary: {text}"
    );
    assert!(
        text.contains("whole.wav  vs  lead.wav"),
        "the two files are named by file name: {text}"
    );
    assert!(text.contains("peak"), "field by field: {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// `audio_level` is gone: the one tool answers both questions.
#[test]
fn audio_level_is_no_longer_a_tool() {
    let reply = call("audio_level", json!({ "project": "x", "file": "x.wav" }));
    let message = reply["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("no tool `audio_level`"), "{reply}");
}
