//! `project_write` measuring what it adds (#783): the way import probes what
//! it brings in, a write probes the assets it names a file for and nobody has
//! measured — before validating, so a clip is held to the source it really is.

use std::path::Path;

use super::fixture::{fingerprint, project, with_footage};
use crate::{call, said};
use scorsese_render::Tools;
use serde_json::{Value, json};

/// `seconds` of a solid colour at `at`: a real file for ffprobe to measure.
fn footage(at: &Path, seconds: &str) {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH for these tests");
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg(format!("color=c=red:s=64x64:d={seconds}:r=30"))
        .args(["-pix_fmt", "yuv420p"])
        .arg(at)
        .output()
        .expect("run ffmpeg");
    assert!(output.status.success(), "ffmpeg failed making a fixture");
}

/// Writes `document` into the project in `dir`, as a client would.
fn write(dir: &Path, document: &str, reprobe: bool) -> (String, bool) {
    said(&call(
        "project_write",
        json!({ "project": dir, "document": document,
                "fingerprint": fingerprint(dir), "reprobe": reprobe }),
    ))
}

/// The boat's `media` block as it is on disk.
fn recorded(dir: &Path) -> Value {
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    let document: Value = serde_json::from_str(&on_disk).expect("the document is JSON");
    let assets = document["assets"].as_array().expect("an assets table");
    let boat = assets.iter().find(|asset| asset["id"] == "boat");
    boat.expect("the boat was written")["media"].clone()
}

#[test]
fn footage_added_by_a_write_is_measured_as_it_is_written() {
    let dir = project("probe-written");
    footage(&dir.join("assets/boat.mp4"), "2");
    let (text, failed) = write(&dir, &with_footage(""), false);
    assert!(!failed, "{text}");
    assert!(text.contains("probed 1 asset(s)"), "got {text}");
    let media = recorded(&dir);
    assert_eq!(media["width"], json!(64), "got {media}");
    let seconds = media["duration_seconds"].as_f64().expect("a length");
    assert!((seconds - 2.0).abs() < 0.1, "got {media}");

    // Measured once: the next write finds nothing new and spawns nothing.
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    let (text, failed) = write(&dir, &document, false);
    assert!(!failed && !text.contains("probed"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// The reason the probe comes before validation: a clip a second long on half
/// a second of footage is refused now, rather than written as a document the
/// first probe afterwards would make unopenable.
#[test]
fn a_clip_longer_than_the_footage_it_turns_out_to_be_is_refused() {
    let dir = project("probe-overrun");
    footage(&dir.join("assets/boat.mp4"), "0.5");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");
    let (text, failed) = write(&dir, &with_footage(""), false);
    assert!(failed, "an overrunning clip must be refused: {text}");
    assert!(text.contains("nothing written"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A file ffprobe cannot read, or one that is not there, is named — and the
/// rest is written, with no half-filled `media` block.
#[test]
fn a_file_that_cannot_be_measured_is_named_and_the_write_goes_ahead() {
    let dir = project("probe-unreadable");
    std::fs::write(dir.join("assets/boat.mp4"), b"not a video").expect("write the fake media");
    let (text, failed) = write(&dir, &with_footage(""), false);
    assert!(!failed, "{text}");
    assert!(text.contains("boat: could not probe"), "got {text}");
    assert_eq!(recorded(&dir), Value::Null);

    std::fs::remove_file(dir.join("assets/boat.mp4")).expect("remove the fake media");
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    let (text, failed) = write(&dir, &document, false);
    assert!(!failed, "{text}");
    assert!(text.contains("boat: its file is not there"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// What `project_probe`'s `all` did, kept reachable: metadata already recorded
/// is left alone unless `reprobe` asks for it to be measured again.
#[test]
fn reprobe_replaces_metadata_that_is_wrong() {
    let dir = project("probe-again");
    footage(&dir.join("assets/boat.mp4"), "2");
    let wrong = with_footage(r#", "media": { "duration_seconds": 99.0 }"#);

    let (text, failed) = write(&dir, &wrong, false);
    assert!(!failed, "{text}");
    assert_eq!(recorded(&dir)["duration_seconds"], json!(99.0));

    let (text, failed) = write(&dir, &wrong, true);
    assert!(!failed && text.contains("probed 1 asset(s)"), "got {text}");
    let seconds = recorded(&dir)["duration_seconds"]
        .as_f64()
        .expect("a length");
    assert!((seconds - 2.0).abs() < 0.1, "got {seconds}");
    std::fs::remove_dir_all(dir).ok();
}
