//! `still` with `sheet: true`: several instants answered with one picture (#814).
//!
//! The wire again, as `seeing` asserts it, plus the one fact about the pixels
//! that the wire carries: a sheet of three cells is three cells wide, and one
//! label strip taller than a cell (#919) — 12 rows under a 160x90 cell, 0.13 of
//! its shorter side rounded to even. Which
//! frame lands in which cell is the tiling's own test, in the compositor.

use super::fixture::project;
use crate::{call, said};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

/// Each cell's raster: small, because every one of these composites for real.
const CELL: &str = "160x90";

/// The width and height a PNG's header declares.
fn png_size(png: &[u8]) -> (u32, u32) {
    let word = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    (word(16), word(20))
}

/// The one picture of a reply, decoded, after checking it is the only one.
fn the_picture(reply: &Value) -> Vec<u8> {
    let content = reply["result"]["content"]
        .as_array()
        .unwrap_or_else(|| panic!("no content in {reply}"));
    assert_eq!(content.len(), 2, "one sentence and one picture: {reply}");
    assert_eq!(content[1]["type"], "image");
    STANDARD
        .decode(content[1]["data"].as_str().expect("base64 data"))
        .expect("valid base64")
}

#[test]
fn a_sheet_answers_several_instants_with_one_picture_in_the_order_asked() {
    let dir = project("still-sheet");
    let reply = call(
        "still",
        json!({ "project": dir, "at": ["1.0s", "0", "599"], "resolution": CELL, "sheet": true }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    let (first, second, third) = (
        text.find("frame 30").expect("names frame 30"),
        text.find("frame 0").expect("names frame 0"),
        text.find("frame 599").expect("names frame 599"),
    );
    assert!(
        first < second && second < third,
        "cells are named in the order asked: {text}"
    );
    assert_eq!(
        png_size(&the_picture(&reply)),
        (3 * 160, 90 + 12),
        "three cells, one row, each whole above its label"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A sheet is one file, so a path with a list is kept rather than refused.
#[test]
fn a_sheet_with_a_path_writes_exactly_that_one_file() {
    let dir = project("still-sheet-out");
    let kept = dir.join("review/sheet.png");
    std::fs::create_dir_all(kept.parent().expect("a parent")).expect("review dir");
    let reply = call(
        "still",
        json!({ "project": dir, "at": ["0", "30"], "resolution": CELL,
                "sheet": true, "out": "review/sheet.png" }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(text.contains("written to review/sheet.png"), "{text}");
    let on_disk = std::fs::read(&kept).expect("the sheet was kept");
    assert_eq!(png_size(&on_disk), (2 * 160, 90 + 12));
    let written: Vec<_> = std::fs::read_dir(kept.parent().expect("a parent"))
        .expect("review dir")
        .collect();
    assert_eq!(written.len(), 1, "one file, not one per instant");
    std::fs::remove_dir_all(dir).ok();
}

/// More than a sheet holds is refused up front, saying the cap.
#[test]
fn a_sheet_of_more_than_five_instants_is_refused() {
    let dir = project("still-sheet-many");
    let reply = call(
        "still",
        json!({ "project": dir, "at": ["0", "1", "2", "3", "4", "5"],
                "resolution": CELL, "sheet": true }),
    );
    let (text, failed) = said(&reply);
    assert!(failed, "six cells must be refused, and got: {text}");
    assert!(
        text.contains("at most 5"),
        "the refusal says the cap: {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// With no `resolution`, a vertical edit is looked at upright and at preview
/// size (#827): the cells take the shape of the first sized clip, here a
/// sketched 9:16 shot, and the reply says that is where it came from.
#[test]
fn a_vertical_edit_is_previewed_upright_without_being_asked() {
    let dir = project("still-sheet-tall");
    let document = super::fixture::DOCUMENT
        .replace(
            r#"{ "id": "title", "kind": "text", "text": "TEASER" },"#,
            r#"{ "id": "title", "kind": "text", "text": "TEASER" },
    { "id": "shot", "kind": "generated_video", "prompt": "a pier", "state": "sketch",
      "video": { "aspect": "9:16" } },"#,
        )
        .replace(r#""asset": "title""#, r#""asset": "shot""#);
    assert!(document.contains(r#""asset": "shot""#), "the fixture moved");
    std::fs::write(dir.join("project.json"), document).expect("rewrite project.json");
    let reply = call("still", json!({ "project": dir, "at": "0", "sheet": true }));
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(text.contains("the shape of shot"), "{text}");
    // 46 rows of label: 0.13 of the 360-pixel side, rounded to even.
    assert_eq!(png_size(&the_picture(&reply)), (360, 640 + 46));
    std::fs::remove_dir_all(dir).ok();
}
