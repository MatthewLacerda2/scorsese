//! Real project directories, and the real binary run over them.
//!
//! These tests go through the compiled `scorsese` rather than calling the
//! function, because the exit code is the thing an unattended caller branches
//! on and only a process has one.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::{PROJECT_FILE_NAME, SCHEMA_VERSION};

/// SHA-256 of nothing, so an empty file matches this recorded hash exactly.
pub(crate) const EMPTY_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// What one `scorsese check` run said and whether it failed.
pub(crate) struct Run {
    pub(crate) failed: bool,
    /// Both streams, joined: problems go to stderr and everything else to
    /// stdout, and a test cares about the whole report.
    pub(crate) output: String,
}

impl Run {
    #[track_caller]
    pub(crate) fn says(&self, needle: &str) {
        assert!(
            self.output.contains(needle),
            "expected `{needle}` in:\n{}",
            self.output
        );
    }

    #[track_caller]
    pub(crate) fn silent_about(&self, needle: &str) {
        assert!(
            !self.output.contains(needle),
            "did not expect `{needle}` in:\n{}",
            self.output
        );
    }
}

pub(crate) fn check(project_dir: &Path, verify: bool) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scorsese"));
    command.arg("check").arg("--project").arg(project_dir);
    if verify {
        command.arg("--verify");
    }
    let output = command.output().expect("run scorsese check");
    Run {
        failed: !output.status.success(),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

/// A project directory holding one video track of `clips` over `assets`.
pub(crate) fn project(label: &str, assets: &[String], clips: &[String]) -> PathBuf {
    let dir = temp_dir(label);
    let document = format!(
        r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "probe",
              "timeline_fps": {{ "num": 30, "den": 1 }},
              "assets": [{}],
              "tracks": [{{ "id": "v1", "kind": "video", "clips": [{}] }}] }}"#,
        assets.join(","),
        clips.join(",")
    );
    std::fs::write(dir.join(PROJECT_FILE_NAME), document).expect("write project.json");
    dir
}

/// An imported video, hashed and probed, pointing at `assets/{id}.mp4`.
pub(crate) fn video(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "video", "path": "assets/{id}.mp4",
              "sha256": "{EMPTY_SHA256}",
              "media": {{ "duration_seconds": 4.0, "width": 640, "height": 360 }} }}"#
    )
}

/// The same, never probed — present on disk but under-described.
pub(crate) fn unprobed_video(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "video", "path": "assets/{id}.mp4",
              "sha256": "{EMPTY_SHA256}" }}"#
    )
}

/// A Veo prompt nobody has spent money on yet: the normal pre-GO shape.
pub(crate) fn sketch(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "generated_video", "prompt": "a boat", "state": "sketch" }}"#
    )
}

pub(crate) fn clip(id: &str, asset: &str, start: u32) -> String {
    format!(r#"{{ "id": "{id}", "asset": "{asset}", "start": {start}, "duration": 30 }}"#)
}

/// Creates the file `video(id)` points at. Empty, so it hashes to what was
/// recorded; `bytes` of anything else makes it a mismatch.
pub(crate) fn media_file(dir: &Path, id: &str, bytes: &[u8]) {
    let assets = dir.join("assets");
    std::fs::create_dir_all(&assets).expect("create assets dir");
    std::fs::write(assets.join(format!("{id}.mp4")), bytes).expect("write media file");
}

fn temp_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-check-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp project dir");
    dir
}
