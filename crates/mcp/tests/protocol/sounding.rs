//! A render delivered as sound alone, and what the reply says about its level.
//!
//! One fixture answers both: a square wave at −1 dBFS under a two-second title,
//! which AAC rebuilds more than 3 dB over full scale (#503's own fixture). Into
//! an `.m4a` it proves the MCP tool reaches a sound-only format (#505), and
//! that the reply says the soundtrack was turned down to fit — the thing an
//! agent cannot hear for itself (#519).

use std::path::Path;

use super::fixture::{DOCUMENT, project};
use crate::{call, said};
use scorsese_render::Tools;
use serde_json::{Value, json};

/// The fixture with its music bed swapped for a square wave on disk, and every
/// clip cut to two seconds so the encode stays cheap.
fn overshooting(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let tools = Tools::discover().expect("ffmpeg must be on PATH for these tests");
    let wave = "0.891*sgn(sin(2*PI*220*t))";
    let status = tools
        .ffmpeg()
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg(format!(
            "aevalsrc=exprs={wave}|{wave}:duration=2:sample_rate=48000"
        ))
        .arg(dir.join("assets/tone.wav"))
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "generating the square wave");
    let bed = r#"{ "id": "bed", "kind": "synth_audio", "recipe": "recipes/bed.json", "state": "sketch" }"#;
    assert!(DOCUMENT.contains(bed), "the fixture no longer has this bed");
    let document = DOCUMENT
        .replace(
            bed,
            r#"{ "id": "bed", "kind": "audio", "path": "assets/tone.wav" }"#,
        )
        .replace("\"duration\": 600", "\"duration\": 60");
    std::fs::write(dir.join("project.json"), document).expect("write the project");
    dir
}

/// ffprobe's codec_type for every stream in the file.
fn streams(file: &Path) -> Vec<String> {
    let tools = Tools::discover().expect("ffprobe must be on PATH for these tests");
    let output = tools
        .ffprobe()
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "json",
        ])
        .arg(file)
        .output()
        .expect("run ffprobe");
    let report: Value = serde_json::from_slice(&output.stdout).expect("ffprobe json parses");
    report["streams"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|stream| stream["codec_type"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn a_soundtrack_is_delivered_alone_and_says_it_was_turned_down() {
    let dir = overshooting("sound-m4a");
    let out = dir.join("score.m4a");
    let (text, failed) = said(&call("render", json!({ "project": dir, "out": out })));
    let found = streams(&out);
    std::fs::remove_dir_all(&dir).ok();

    assert!(!failed, "render refused: {text}");
    assert!(text.contains("m4a (aac, sound only)"), "{text}");
    assert!(text.contains("of sound, no picture"), "{text}");
    assert_eq!(found, ["audio"], "one stream, and it is sound");
    assert!(
        text.contains("\nfile   "),
        "the delivered level is said: {text}"
    );
    assert!(
        text.contains("note: the soundtrack was turned down"),
        "the cut is said, in the CLI's words: {text}"
    );
}
