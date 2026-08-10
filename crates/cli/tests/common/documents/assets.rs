//! Rows of the assets table, one function per state worth testing.
//!
//! Each is a `String` of JSON rather than an [`scorsese_core::Asset`], for the
//! reason the parent module gives: the document is the contract.

use super::EMPTY_SHA256;

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

/// A file-backed asset with nothing recorded about it: no hash, no metadata,
/// just a path. What a render is handed when it has to find out for itself,
/// and the shape the media fixtures generate real files for.
pub(crate) fn raw(id: &str, kind: &str, extension: &str) -> String {
    format!(r#"{{ "id": "{id}", "kind": "{kind}", "path": "assets/{id}.{extension}" }}"#)
}

/// A Veo prompt nobody has spent money on yet: the normal pre-GO shape.
pub(crate) fn sketch(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "generated_video", "prompt": "a boat", "state": "sketch" }}"#
    )
}

/// An ElevenLabs prompt in the same state, which belongs on an audio track.
pub(crate) fn narration(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "generated_audio",
              "prompt": "and then the boat sank", "state": "sketch" }}"#
    )
}

/// A prompt the project believes was generated, pointing at a file under
/// `generated/` that is not there — the state a deleted cache leaves behind.
pub(crate) fn generated(id: &str, kind: &str, extension: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "generated_{kind}", "prompt": "a boat",
              "state": "generated", "path": "generated/{id}.{extension}" }}"#
    )
}

/// A prompt edited after it generated: the state says so, and the path still
/// points at the *previous* generation's file — which may well still exist.
pub(crate) fn stale(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "generated_video", "prompt": "a different boat",
              "state": "stale", "path": "generated/{id}.mp4" }}"#
    )
}

/// A title: the one kind with no file behind it at all.
pub(crate) fn text(id: &str, content: &str) -> String {
    format!(r#"{{ "id": "{id}", "kind": "text", "text": "{content}" }}"#)
}
