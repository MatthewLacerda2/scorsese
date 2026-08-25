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
        "text_new",
        json!({ "project": dir, "text": "THE VESSEL ARRIVES", "size": 0.08,
                "color": "#ffcc00" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`the-vessel-arrives`"), "got {text}");

    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "the-vessel-arrives", "track": "v1",
                "start_seconds": 25.0, "duration_seconds": 2.0 }),
    ));
    assert!(!failed, "{text}");
    assert!(document(&dir).contains("#ffcc00"), "the style was written");
    std::fs::remove_dir_all(dir).ok();
}

/// An id a caller chose is one it is about to write onto a clip, so a repeat
/// is refused rather than suffixed behind its back.
#[test]
fn an_id_already_in_use_is_refused_and_writes_nothing() {
    let dir = project("text-taken");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "text_new",
        json!({ "project": dir, "text": "AGAIN", "asset": "title" }),
    ));
    assert!(failed, "`title` is the fixture's caption");
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(document(&dir), before);
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
    let (text, failed) = said(&call("color_new", json!({ "project": dir })));
    assert!(failed, "a colour card with no colour");
    assert!(text.contains("`color` is required"), "got {text}");

    let (text, failed) = said(&call(
        "icon_new",
        json!({ "project": dir, "name": "clapperboard", "size": 0.2 }),
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
        "shape_new",
        json!({ "project": dir, "geometry": "arrow", "stroke": "#ffffff",
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
        "shape_new",
        json!({ "project": dir, "geometry": "rectangle", "width": 0.4, "height": 0.2 }),
    ));
    assert!(failed, "no fill and no stroke");
    assert!(text.contains("nothing was written"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
