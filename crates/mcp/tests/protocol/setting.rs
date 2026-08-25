//! Changing one field of an inline asset, over the wire.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// The project document as it is on disk.
fn document(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk")
}

/// **The reason this verb merges rather than replaces.** The caption is set in
/// a serif face; shrinking it must not put the font back to the default, which
/// is what sending a whole style block would do and would say nothing about.
#[test]
fn setting_one_field_leaves_the_rest_of_the_style_alone() {
    let dir = project("set-merge");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "title", "font": "serif", "size": 0.12 }),
    ));
    assert!(!failed, "{text}");

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "title", "size": 0.08 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("size: 0.12 → 0.08"), "got {text}");
    assert!(text.contains("Nothing else changed"), "got {text}");
    assert!(document(&dir).contains("serif"), "the font survived");
    std::fs::remove_dir_all(dir).ok();
}

/// Rewording a caption is the loop this exists for, and the reply says what
/// it replaced — the caller cannot see the document.
#[test]
fn rewording_a_caption_reports_both_halves() {
    let dir = project("set-reword");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "title", "text": "TRAILER" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains(r#""TEASER" → "TRAILER""#), "got {text}");
    assert!(document(&dir).contains("TRAILER"), "and it landed");
    std::fs::remove_dir_all(dir).ok();
}

/// A field the kind has no use for is refused by name. Ignored, it would look
/// exactly like a fill that had been applied.
#[test]
fn a_field_of_another_kind_is_refused_by_name() {
    let dir = project("set-wrong-field");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "title", "fill": "#000000" }),
    ));
    assert!(failed, "a caption has no fill");
    assert!(text.contains("`fill` is not a field"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// What a generated asset is made from is `rebrief`'s, and this says so
/// rather than editing a field nothing would read.
#[test]
fn a_generated_asset_is_not_this_verb() {
    let dir = project("set-generated");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "vo", "text": "a different line" }),
    ));
    assert!(failed, "a brief is rebrief's");
    assert!(
        text.contains("carry their content in the document"),
        "got {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A call that names no field at all is a spelling mistake, not a no-op, and
/// answering "written" to one is how a change goes missing.
#[test]
fn naming_no_field_is_refused() {
    let dir = project("set-nothing");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "title" }),
    ));
    assert!(failed, "nothing was asked for");
    assert!(text.contains("nothing to change"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
