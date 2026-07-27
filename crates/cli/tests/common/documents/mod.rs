//! Building a `project.json` by hand, for the shapes `scorsese new` cannot
//! make: a pool with a file missing from it, a prompt nobody has generated, a
//! soundtrack that outlasts the picture.
//!
//! Written as text rather than serialised from a [`scorsese_core::Project`] on
//! purpose — the document is the contract, and a fixture that goes through our
//! own writer could not express a document our own writer would never produce.

pub(crate) mod assets;

use std::path::{Path, PathBuf};

use scorsese_core::{PROJECT_FILE_NAME, SCHEMA_VERSION};

use super::temp_dir;

/// SHA-256 of nothing, so an empty file matches this recorded hash exactly.
pub(crate) const EMPTY_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// A fresh project directory holding `assets` and `tracks`.
pub(crate) fn written(label: &str, assets: &[String], tracks: &[String]) -> PathBuf {
    let dir = temp_dir(label);
    write_into(&dir, assets, tracks);
    dir
}

/// The same document, into a directory that already exists — so a test can lay
/// a timeline over media that has already been generated into it.
pub(crate) fn write_into(dir: &Path, assets: &[String], tracks: &[String]) {
    let document = format!(
        r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "probe",
              "timeline_fps": {{ "num": 30, "den": 1 }},
              "assets": [{}],
              "tracks": [{}] }}"#,
        assets.join(","),
        tracks.join(",")
    );
    std::fs::write(dir.join(PROJECT_FILE_NAME), document).expect("write project.json");
}

/// A project directory holding one video track of `clips` over `assets` — the
/// commonest shape, and the only one most tests need.
pub(crate) fn project(label: &str, assets: &[String], clips: &[String]) -> PathBuf {
    written(label, assets, &[video_track(clips)])
}

pub(crate) fn video_track(clips: &[String]) -> String {
    track("v1", "video", clips)
}

pub(crate) fn audio_track(clips: &[String]) -> String {
    track("a1", "audio", clips)
}

pub(crate) fn track(id: &str, kind: &str, clips: &[String]) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "{kind}", "clips": [{}] }}"#,
        clips.join(",")
    )
}

/// A one-second clip at 30 fps, which is long enough for a cut and short
/// enough that rendering one costs nothing.
pub(crate) fn clip(id: &str, asset: &str, start: u32) -> String {
    lasting(id, asset, start, 30)
}

pub(crate) fn lasting(id: &str, asset: &str, start: u32, duration: u32) -> String {
    format!(r#"{{ "id": "{id}", "asset": "{asset}", "start": {start}, "duration": {duration} }}"#)
}

/// Creates the file an `assets::video` row points at. Empty, so it hashes to
/// what was recorded; `bytes` of anything else makes it a mismatch.
pub(crate) fn media_file(dir: &Path, id: &str, bytes: &[u8]) {
    file_at(dir, &format!("assets/{id}.mp4"), bytes);
}

/// Writes `bytes` at a path relative to the project root, making the
/// directories above it.
pub(crate) fn file_at(dir: &Path, relative: &str, bytes: &[u8]) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create the parent directory");
    }
    std::fs::write(&path, bytes).expect("write the file");
}
