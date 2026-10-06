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
    assert!(
        text.contains("`fill` is not something a text asset takes"),
        "got {text}"
    );
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// A generated asset takes its brief here and nothing else, and a caption's
/// field on one is refused by name rather than written where nothing reads it.
#[test]
fn a_generated_asset_takes_only_its_brief() {
    let dir = project("set-generated");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "vo", "text": "a different line" }),
    ));
    assert!(failed, "a narration has no `text`");
    assert!(text.contains("it takes prompt, speech"), "got {text}");
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

/// A made shape takes its paint and size afterwards; its outline, ends and
/// dashes were chosen when it was made, and the refusal says so.
#[test]
fn a_shape_changes_its_paint_but_not_its_outline() {
    let dir = project("set-shape");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "asset": "box", "geometry": "rectangle",
                "width": 0.4, "height": 0.2, "fill": "#101820" }),
    ));
    assert!(!failed, "{text}");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "box", "fill": "#ffcc00" }),
    ));
    assert!(!failed, "a shape's fill is a nudge: {text}");
    assert!(document(&dir).contains("#ffcc00"), "the fill landed");

    let before = document(&dir);
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "asset": "box", "dash": [0.02] }),
    ));
    assert!(failed, "a made shape's dashes are not a nudge");
    assert!(
        text.contains("`dash`") && text.contains("once it is made"),
        "got {text}"
    );
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// A client that sends `null` for what it is not saying has said nothing — a
/// `null` block is not a field the kind is asked to take.
#[test]
fn a_null_block_is_not_given() {
    let dir = project("set-null");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "color", "color": "#000000", "reveal": null,
                "speech": null }),
    ));
    assert!(!failed, "{text}");
    std::fs::remove_dir_all(dir).ok();
}
