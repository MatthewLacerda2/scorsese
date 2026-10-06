//! Authoring the assets nothing brings in, and the lanes they sit on.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// The document, for a test that has to see what actually landed in it.
fn document(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk")
}

/// The whole point: a caption authored in one call, then placed, with no
/// project.json crossing the wire in either direction.
#[test]
fn a_caption_is_authored_and_then_placed() {
    let dir = project("text-new");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "text", "text": "THE VESSEL ARRIVES", "size": 0.08,
                "color": "#ffcc00", "stroke": "#101820", "stroke_width": 0.003 }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`the-vessel-arrives`"), "got {text}");

    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "the-vessel-arrives", "track": "v1",
                "start_seconds": 25.0, "duration_seconds": 2.0 }),
    ));
    assert!(!failed, "{text}");
    let written = document(&dir);
    assert!(written.contains("#ffcc00"), "the style was written");
    // The rim is what keeps a burned-in caption legible over footage, and it
    // is a field this verb had to learn rather than one it inherited.
    assert!(
        written.contains("stroke"),
        "the rim came with it: {written}"
    );
    assert!(written.contains("#101820"), "and its colour: {written}");
    std::fs::remove_dir_all(dir).ok();
}

/// A `kind` that is not the asset's is a call that has misunderstood which
/// asset it is writing, so it is refused rather than guessed at.
#[test]
fn a_kind_that_is_not_the_assets_is_refused_and_writes_nothing() {
    let dir = project("text-taken");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "color", "color": "#000000", "asset": "title" }),
    ));
    assert!(
        failed,
        "`title` is the fixture's caption, not a colour card"
    );
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}

/// The same kind is accepted: an agent re-sending a create must not fail on
/// the second try, and what it gets is the change it asked for.
#[test]
fn making_an_id_that_exists_as_that_kind_changes_it() {
    let dir = project("text-again");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "text", "text": "AGAIN", "asset": "title" }),
    ));
    assert!(!failed, "{text}");
    assert!(document(&dir).contains("AGAIN"), "the caption was reworded");
    std::fs::remove_dir_all(dir).ok();
}

/// An id nothing answers to, with no `kind`, cannot be made — and says how.
#[test]
fn an_unknown_id_without_a_kind_says_to_give_one() {
    let dir = project("no-kind");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "nobody", "text": "hi" }),
    ));
    assert!(failed, "nothing to change and nothing said to make");
    assert!(text.contains("`kind`"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// An argument the kind has no use for is refused by name, made or changed —
/// the `*_new` tools ignored one, which is an edit somebody thinks they made.
#[test]
fn an_argument_the_kind_does_not_take_is_refused_by_name() {
    let dir = project("wrong-field");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "text", "text": "HI", "fill": "#ff0000" }),
    ));
    assert!(failed, "a caption has no fill");
    assert!(text.contains("`fill`"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// The refusal `track_new` exists to answer: two things on screen at once
/// need two lanes, and the new video lane composites over the old one.
#[test]
fn a_new_lane_is_where_an_overlapping_clip_goes() {
    let dir = project("track-new");
    let (text, failed) = said(&call(
        "track_new",
        json!({ "project": dir, "kind": "video" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`v2`"), "got {text}");

    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "title", "track": "v2",
                "start_seconds": 5.0, "duration_seconds": 2.0 }),
    ));
    assert!(
        !failed,
        "frames 150-210 of v1 are taken, v2 is empty: {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A colour card and a symbol, each refused without the one thing the format
/// deliberately gives no default.
#[test]
fn the_kinds_with_no_safe_default_say_so() {
    let dir = project("no-default");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "color" }),
    ));
    assert!(failed, "a colour card with no colour");
    assert!(text.contains("`color` is required"), "got {text}");

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "icon", "icon": "clapperboard", "size": 0.2 }),
    ));
    assert!(failed, "a symbol with no colour");
    assert!(text.contains("`color` is required"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// An arrow is its two ends, and an end may be a clip rather than a point —
/// which is what makes a diagram survive its boxes moving.
#[test]
fn an_arrow_may_follow_a_clip() {
    let dir = project("shape-arrow");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "geometry": "arrow", "stroke": "#ffffff",
                "from": { "x": 0.1, "y": 0.5 },
                "to": { "clip": "c1", "side": "left" } }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("an arrow"), "got {text}");
    assert!(
        document(&dir).contains("\"attach\""),
        "the end was attached"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A shape with neither a fill nor a border draws nothing, and a layer that
/// renders nothing looks exactly like one that failed to.
#[test]
fn a_shape_that_would_draw_nothing_is_refused() {
    let dir = project("shape-blank");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "geometry": "rectangle", "width": 0.4, "height": 0.2 }),
    ));
    assert!(failed, "no fill and no stroke");
    assert!(text.contains("nothing was written"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
