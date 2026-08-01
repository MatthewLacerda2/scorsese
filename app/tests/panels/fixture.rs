//! Project directories for the tests to draw and to rewrite.
//!
//! Shared by more than one test binary, and each uses a different slice of it,
//! so items unused from where you are reading are expected rather than dead.
#![allow(dead_code)]
//!
//! Written out as text rather than built through `scorsese-core`, for the same
//! reason `crates/cli`'s fixtures are: the document is the contract, and a
//! fixture that goes through our own writer cannot express a document our own
//! writer would never produce — including the broken one.

use std::path::{Path, PathBuf};

/// A project directory that removes itself when the test ends.
pub(crate) struct Fixture(PathBuf);

impl Fixture {
    /// Where it is.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A whole edit: a title over a colour, music under narration, a duck already
/// written on the music, and two assets nobody has generated.
///
/// Everything a panel can draw, in one project — a snapshot of a project that
/// exercises one case at a time would need six projects and would still miss
/// how they look beside each other.
pub(crate) fn project(label: &str) -> Fixture {
    write(label, DOCUMENT)
}

/// The same project with a clip pointing at an asset that is not there.
pub(crate) fn broken(label: &str) -> Fixture {
    write(
        label,
        &DOCUMENT.replace(r#""asset": "bed""#, r#""asset": "ghost""#),
    )
}

/// Named for the label alone — no process id, no counter.
///
/// The window puts the project directory's name in its menu bar, so a name
/// that changed between runs would change the picture between runs, and a
/// snapshot that cannot reproduce itself is not a reference. Each test uses a
/// different label, so nothing collides inside one run of this binary.
fn write(label: &str, document: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("scorsese-panels-{label}.scor"));
    let _ = std::fs::remove_dir_all(&dir);
    for inside in ["assets", "generated", "recipes", "cache"] {
        std::fs::create_dir_all(dir.join(inside)).expect("create the project directory");
    }
    std::fs::write(dir.join("project.json"), document).expect("write project.json");
    Fixture(dir)
}

const DOCUMENT: &str = r##"{
  "schema_version": 12,
  "name": "Narrated teaser",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "title", "kind": "text", "text": "CHAPTER ONE",
      "style": { "font": "serif", "size": 0.14, "color": "#f5f0e6" } },
    { "id": "shot-city", "kind": "generated_video", "state": "sketch",
      "prompt": "wide aerial of a city at dawn, slow push in" },
    { "id": "vo", "kind": "generated_audio", "state": "sketch",
      "prompt": "In a city that never sleeps, one editor never blinks." },
    { "id": "bed", "kind": "synth_audio", "state": "sketch",
      "recipe": "recipes/bed.json" }
  ],
  "tracks": [
    { "id": "v1", "kind": "video", "name": "Main", "clips": [
      { "id": "c-shot", "asset": "shot-city", "start": 0, "duration": 300 },
      { "id": "c-title", "asset": "title", "start": 300, "duration": 120 } ] },
    { "id": "a1", "kind": "audio", "name": "Music", "clips": [
      { "id": "c-bed", "asset": "bed", "start": 0, "duration": 420,
        "keyframes": [
          { "property": "volume", "by": "duck", "keyframes": [
            { "t": 51, "value": 1.0, "easing": "ease_out" },
            { "t": 60, "value": 0.25 },
            { "t": 230, "value": 0.25, "easing": "ease_in" },
            { "t": 248, "value": 1.0 } ] } ] } ] },
    { "id": "a2", "kind": "audio", "name": "Narration", "clips": [
      { "id": "c-vo", "asset": "vo", "start": 60, "duration": 170 } ] }
  ]
}
"##;
