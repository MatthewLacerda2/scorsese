//! What kind of file `render` delivers, asked of the file rather than the reply.
//!
//! The tool once wrote mp4/H.264/AAC whatever `out` was called — an `.avi` that
//! ffprobe read as an mp4 — and said only `wrote cut.avi`. So the assertion
//! here is ffprobe's, not the server's: the reply is the claim, the file is the
//! evidence. The whole container table is `scorsese-render`'s tests; what is
//! held here is that the MCP tool reaches it the way the CLI does.

use std::path::Path;

use super::fixture::project;
use crate::{call, said};
use scorsese_render::Tools;
use serde_json::{Value, json};

/// Three frames at a postage stamp: what is asked is what kind of file came
/// out, and every call here is a real encode.
fn render(dir: &Path, extra: Value) -> (String, bool) {
    let mut arguments = json!({ "project": dir, "resolution": "160x90", "range": "0:3" });
    for (key, value) in extra.as_object().expect("extra arguments are an object") {
        arguments[key] = value.clone();
    }
    said(&call("render", arguments))
}

/// ffprobe's container list and the first video stream's codec.
fn probe(file: &Path) -> (String, String) {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH for these tests");
    let output = tools
        .ffprobe()
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
        .args(["format=format_name:stream=codec_name", "-of", "json"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    assert!(output.status.success(), "ffprobe could not read {file:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("ffprobe json parses");
    let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    (
        text(&report["format"]["format_name"]),
        text(&report["streams"][0]["codec_name"]),
    )
}

/// The file the bug was found with: an `.avi` is an AVI, carrying MPEG-4 Part
/// 2 as docs/output-formats.md says, and the reply names the format it wrote.
#[test]
fn an_avi_is_delivered_as_an_avi() {
    let dir = project("deliver-avi");
    let out = dir.join("cut.avi");
    let (text, failed) = render(&dir, json!({ "out": out }));
    assert!(!failed, "render refused: {text}");
    assert!(text.contains("avi (mpeg4 + pcm_s16le)"), "got {text}");
    // What the CLI prints about sound, the reply says too (#519).
    assert!(
        text.contains("\nfile   "),
        "the delivered level is said: {text}"
    );
    assert_eq!(probe(&out), ("avi".to_owned(), "mpeg4".to_owned()));
    std::fs::remove_dir_all(dir).ok();
}

/// The overrides the CLI has as flags: a named container beats the extension,
/// and a named codec beats the container's own.
#[test]
fn a_named_container_and_codec_win_over_the_file_name() {
    let dir = project("deliver-override");
    let out = dir.join("cut.mp4");
    let (text, failed) = render(
        &dir,
        json!({ "out": out, "container": "avi", "video_codec": "h264" }),
    );
    assert!(!failed, "render refused: {text}");
    assert_eq!(probe(&out), ("avi".to_owned(), "h264".to_owned()));
    std::fs::remove_dir_all(dir).ok();
}

/// Refused in the CLI's words — both build the format with
/// `OutputFormat::for_path` — and before anything is written.
#[test]
fn a_format_scorsese_does_not_write_is_refused_the_way_the_command_line_refuses_it() {
    let dir = project("deliver-refused");
    for (extra, says) in [
        (
            json!({ "out": dir.join("cut.wmv"), "video_codec": "h264" }),
            "scorsese does not write wmv with h264; it writes wmv with: wmv2",
        ),
        (
            json!({ "out": dir.join("cut.mov") }),
            "scorsese does not write `.mov` files",
        ),
        (
            json!({ "out": dir.join("cut") }),
            "has no extension to take an output format from",
        ),
        (
            json!({ "out": dir.join("cut.mp4"), "audio_codec": "opus" }),
            "`opus` is not an audio codec scorsese writes",
        ),
        (
            json!({ "out": dir.join("score.mp3"), "resolution": "160x90" }),
            "mp3 carries sound only, so a resolution has no picture to apply to",
        ),
        (
            json!({ "out": dir.join("score.wav"), "video_codec": "h264" }),
            "wav carries sound only, so a video codec has no picture to apply to",
        ),
    ] {
        let (text, failed) = render(&dir, extra.clone());
        assert!(failed, "{extra} must be refused, and got: {text}");
        assert!(text.contains(says), "{extra} said the wrong thing: {text}");
    }
    for name in [
        "cut.wmv",
        "cut.mov",
        "cut",
        "cut.mp4",
        "score.mp3",
        "score.wav",
    ] {
        assert!(!dir.join(name).exists(), "a refusal left {name} behind");
    }
    std::fs::remove_dir_all(dir).ok();
}
