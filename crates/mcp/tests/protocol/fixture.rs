//! A project directory for the tool tests to work on.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// A project directory with a title over a music bed and a line of narration.
///
/// A title rather than footage because a `text` asset needs no file on disk,
/// so the fixture is one document and nothing else — and a project with no
/// picture at all cannot be described, which is the renderer being right
/// rather than a gap worth working around.
pub(crate) fn project(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-mcp-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the project directory");
    std::fs::write(dir.join("project.json"), DOCUMENT).expect("write project.json");
    dir
}

pub(crate) const DOCUMENT: &str = r#"{
  "schema_version": 6,
  "name": "Teaser",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "title", "kind": "text", "text": "TEASER" },
    { "id": "vo", "kind": "generated_audio", "prompt": "a line", "state": "sketch" },
    { "id": "bed", "kind": "synth_audio", "recipe": "recipes/bed.json", "state": "sketch" }
  ],
  "tracks": [
    { "id": "v1", "kind": "video",
      "clips": [ { "id": "c1", "asset": "title", "start": 0, "duration": 600 } ] },
    { "id": "music", "kind": "audio",
      "clips": [ { "id": "m1", "asset": "bed", "start": 0, "duration": 600 } ] },
    { "id": "vo", "kind": "audio",
      "clips": [ { "id": "v1c", "asset": "vo", "start": 60, "duration": 90 } ] }
  ]
}
"#;
