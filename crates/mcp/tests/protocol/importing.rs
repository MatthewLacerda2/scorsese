//! Bringing media in from outside the project.
//!
//! This is the one tool whose argument names a path the project knows nothing
//! about, so what is asserted is the bargain that makes that safe: the bytes
//! are **copied** into `assets/`, the document records the relative path they
//! landed at, and the outside path is never written down anywhere.

use std::path::{Path, PathBuf};

use super::fixture::project;
use crate::{call, said};
use scorsese_render::Tools;
use serde_json::json;

/// A folder outside any project, holding two clips, a licence file and a
/// subfolder — the shape a folder of footage actually arrives in.
fn footage(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scorsese-mcp-{label}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create the source folder");
    clip(&dir.join("wide.mp4"), "red");
    clip(&dir.join("close.mp4"), "blue");
    clip(&dir.join("b-roll/extra.mp4"), "green");
    std::fs::write(dir.join("LICENCE.txt"), b"a font's licence").expect("write the licence");
    dir
}

/// Two seconds of a solid colour: a real file, so ffprobe has something to
/// measure and the reply can say what it measured.
fn clip(at: &Path, colour: &str) {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH for these tests");
    std::fs::create_dir_all(at.parent().expect("a clip has a parent directory"))
        .expect("create the clip's directory");
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg(format!("color=c={colour}:s=64x64:d=2:r=30"))
        .args(["-pix_fmt", "yuv420p"])
        .arg(at)
        .output()
        .expect("run ffmpeg");
    assert!(output.status.success(), "ffmpeg failed making a fixture");
}

#[test]
fn a_folder_of_footage_comes_in_one_asset_at_a_time() {
    let dir = project("import");
    let outside = footage("import-sources");

    let (text, failed) = said(&call("import", json!({ "project": dir, "path": outside })));

    assert!(!failed, "{text}");
    assert!(text.contains("close — Video, 64x64"), "got {text}");
    assert!(text.contains("wide — Video, 64x64"), "got {text}");
    assert!(text.contains("LICENCE.txt — skipped"), "got {text}");
    assert!(text.contains("b-roll — skipped"), "got {text}");

    let document = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(document.contains(r#""assets/wide.mp4""#), "got {document}");
    assert!(
        !document.contains(outside.to_str().expect("a utf-8 fixture path")),
        "the outside path must never be written down: {document}"
    );
    assert!(dir.join("assets/close.mp4").is_file(), "it was copied in");
    assert!(outside.join("wide.mp4").is_file(), "the original is left");

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

/// The contract `project_write` and `scale_pacing` already keep: a refusal is
/// a successful call whose answer is no, and it changes nothing at all.
#[test]
fn an_id_an_asset_already_answers_to_is_refused_with_nothing_copied() {
    let dir = project("import-taken");
    let outside = footage("import-taken-sources");
    // `title` is already an asset in the fixture document.
    std::fs::rename(outside.join("wide.mp4"), outside.join("title.mp4")).expect("rename");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");

    let (text, failed) = said(&call("import", json!({ "project": dir, "path": outside })));

    assert!(failed, "`title` is taken, so this must be refused: {text}");
    assert!(text.contains("already an asset"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read back"),
        before,
        "the document must be untouched"
    );
    assert_eq!(
        std::fs::read_dir(dir.join("assets")).expect("read").count(),
        0,
        "and nothing may have been copied in"
    );

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

#[test]
fn a_kind_that_carries_no_file_is_refused_by_name() {
    let dir = project("import-kind");
    let (text, failed) = said(&call(
        "import",
        json!({ "project": dir, "path": "/nowhere.mp4", "kind": "text" }),
    ));
    assert!(
        failed && text.contains("video, image or audio"),
        "got {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_list_of_paths_comes_in_from_one_call() {
    // The round trip is the point: an assistant handed five files should pay
    // for one call, not five.
    let dir = project("import-list");
    let outside = footage("import-list-sources");
    let named = |name: &str| outside.join(name).to_str().expect("utf-8").to_owned();

    let (text, failed) = said(&call(
        "import",
        json!({ "project": dir, "path": [named("wide.mp4"), named("close.mp4")] }),
    ));

    assert!(!failed, "{text}");
    assert!(text.contains("wide — Video, 64x64"), "got {text}");
    assert!(text.contains("close — Video, 64x64"), "got {text}");
    assert!(dir.join("assets/wide.mp4").is_file(), "wide was copied in");
    assert!(
        dir.join("assets/close.mp4").is_file(),
        "close was copied in"
    );

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

#[test]
fn one_bad_path_among_good_ones_keeps_the_good_ones() {
    // A mistyped name in the middle of a set is a typo, not a reason to import
    // nothing — and the reply still has to say what did not land.
    let dir = project("import-partial");
    let outside = footage("import-partial-sources");
    let named = |name: &str| outside.join(name).to_str().expect("utf-8").to_owned();

    let (text, failed) = said(&call(
        "import",
        json!({ "project": dir, "path": [named("wide.mp4"), named("nope.mp4")] }),
    ));

    assert!(!failed, "something landed, so this is a reply: {text}");
    assert!(text.contains("wide — Video, 64x64"), "got {text}");
    assert!(text.contains("nope.mp4 — failed"), "got {text}");
    assert!(dir.join("assets/wide.mp4").is_file(), "wide still landed");

    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

#[test]
fn every_path_failing_is_still_an_error() {
    let dir = project("import-none");
    let (text, failed) = said(&call(
        "import",
        json!({ "project": dir, "path": ["/nowhere-a.mp4", "/nowhere-b.mp4"] }),
    ));
    assert!(failed, "nothing landed, so this is an error: {text}");
    assert!(text.contains("nothing was imported"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
