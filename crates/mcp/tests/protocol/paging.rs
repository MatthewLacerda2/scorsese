//! Web pages: written whole, read back, and looked at with `still` — whose
//! notes are how an agent hears what its page could not do.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

const PAGE: &str = "<!doctype html><div style='position:absolute;inset:0;background:#08f'>\
                    </div><img src='https://example.com/logo.png'>";

/// One name, both cases: a new one makes the asset and its file, the same one
/// again rewrites the file and leaves the document alone.
#[test]
fn page_write_makes_a_page_and_then_rewrites_it() {
    let dir = project("page-write");
    let (text, failed) = said(&call(
        "page_write",
        json!({ "project": dir, "page": "lower-third", "html": "<p>first</p>" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("pages/lower-third.html"), "got {text}");
    let document = std::fs::read_to_string(dir.join("project.json")).expect("document");
    assert!(document.contains("\"kind\": \"html\""), "{document}");

    let (text, failed) = said(&call(
        "page_write",
        json!({ "project": dir, "page": "lower-third", "html": "<p>second</p>" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("rewritten"), "got {text}");
    let after = std::fs::read_to_string(dir.join("project.json")).expect("document");
    assert_eq!(after, document, "a rewrite changes nothing in the document");

    let (read_back, failed) = said(&call(
        "page_read",
        json!({ "project": dir, "page": "lower-third" }),
    ));
    assert!(!failed, "{read_back}");
    assert_eq!(read_back, "<p>second</p>");
    std::fs::remove_dir_all(dir).ok();
}

/// The fixture's `title` is a text asset: writing a page over it is refused,
/// and so is reading it as one.
#[test]
fn a_name_that_is_not_a_page_is_refused_both_ways() {
    let dir = project("page-not");
    let (text, failed) = said(&call(
        "page_write",
        json!({ "project": dir, "page": "title", "html": "<p>no</p>" }),
    ));
    assert!(failed, "{text}");
    assert!(
        text.contains("nothing written") && text.contains("`text`"),
        "{text}"
    );
    assert!(!dir.join("pages").exists(), "no file landed");

    let (text, failed) = said(&call(
        "page_read",
        json!({ "project": dir, "page": "title" }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("not a page"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A page that asks the internet for something is drawn without it, and both
/// the frame and the render come back with a note that says so (#777). Needs
/// the pinned browser.
#[test]
fn a_still_and_a_render_of_a_page_carry_its_warnings() {
    let dir = project("page-still");
    said(&call(
        "page_write",
        json!({ "project": dir, "page": "card", "html": PAGE }),
    ));
    call("track_new", json!({ "project": dir, "kind": "video" }));
    let (text, failed) = said(&call(
        "place_clip",
        json!({ "project": dir, "asset": "card", "track": "v2",
                "start_seconds": 0.0, "duration_seconds": 1.0 }),
    ));
    assert!(!failed, "{text}");

    let (text, failed) = said(&call(
        "still",
        json!({ "project": dir, "at": ["0.2s", "0.5s"], "resolution": "160x90" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("note: page `card`") && text.contains("example.com"),
        "the refused request is told: {text}"
    );
    assert_eq!(text.matches("note:").count(), 1, "once a call: {text}");

    let (text, failed) = said(&call(
        "render",
        json!({ "project": dir, "out": "out.mp4", "resolution": "160x90", "wait": true }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("note: page `card`"),
        "the render says it too: {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}
