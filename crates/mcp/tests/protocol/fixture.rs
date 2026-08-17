//! A project directory for the tool tests to work on.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// The fingerprint `project_read` reports for the project in `dir`, taken the
/// way a client takes it — out of the reply rather than computed beside the
/// server. It is what `project_write` has to be handed.
pub(crate) fn fingerprint(dir: &std::path::Path) -> String {
    let reply = crate::call("project_read", serde_json::json!({ "project": dir }));
    let note = reply["result"]["content"][1]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("project_read reported no fingerprint: {reply}"));
    note.trim_start_matches("fingerprint: ")
        .split_whitespace()
        .next()
        .expect("a fingerprint is one word")
        .to_owned()
}

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
    // The directories `scorsese new` makes, because a hand-written fixture
    // that skips them is testing a project shape nothing produces.
    for inside in ["assets", "generated", "recipes", "cache"] {
        std::fs::create_dir_all(dir.join(inside)).expect("create the project directory");
    }
    std::fs::write(dir.join("project.json"), DOCUMENT).expect("write project.json");
    // The bed's recipe really exists: a synth asset pointing at a recipe that
    // is not there is a broken project, not a starting point.
    std::fs::write(dir.join("recipes/bed.json"), BED).expect("write the bed recipe");
    dir
}

/// The music bed: one noise blip, sounded once and left to ring out for the
/// length of the cut. Cheap to render and audible.
///
/// It runs the whole twenty seconds `m1` plays it for, because a baked asset
/// is a measured one and a clip may not reach past the media it shows. A
/// two-tenths-of-a-second bed under a twenty-second title is a project that
/// does not validate, not a small fixture.
pub(crate) const BED: &str = r#"{
  "recipe": "patch",
  "note": "C3",
  "duration": 20.0,
  "patch": {
    "source": { "kind": "noise" },
    "amp": { "a": 0.001, "d": 0.08, "s": 0.0, "r": 0.05 }
  }
}
"#;

/// SHA-256 of nothing, so an empty file matches this recorded hash exactly
/// and a file with anything in it does not.
pub(crate) const EMPTY_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The fixture with a shot of footage cut into it, whose asset row carries
/// `extra` — a recorded hash, or nothing at all.
///
/// The file itself is not written here: whether it is there, and whether it is
/// still what was imported, is what the test is about.
pub(crate) fn with_footage(extra: &str) -> String {
    let document = DOCUMENT
        .replace(
            r#"{ "id": "title", "kind": "text", "text": "TEASER" },"#,
            &format!(
                r#"{{ "id": "title", "kind": "text", "text": "TEASER" }},
    {{ "id": "boat", "kind": "video", "path": "assets/boat.mp4"{extra} }},"#
            ),
        )
        .replace(
            r#"[ { "id": "c1", "asset": "title", "start": 0, "duration": 600 } ]"#,
            r#"[ { "id": "c1", "asset": "title", "start": 0, "duration": 600 },
                 { "id": "c2", "asset": "boat", "start": 600, "duration": 30 } ]"#,
        );
    // The document is edited by matching text, so a reworded fixture would
    // otherwise leave these tests quietly checking the fixture instead.
    assert!(
        document.contains("boat") && document.contains("c2"),
        "the fixture no longer contains what this edits"
    );
    document
}

pub(crate) const DOCUMENT: &str = r#"{
  "schema_version": 24,
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
