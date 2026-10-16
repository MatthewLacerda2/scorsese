use serde_json::{Value, json};

use crate::frame::Ratio;
use crate::migrate::parse;

/// A v51 document with `tracks` laid out as given, over the assets given.
fn v51(assets: Value, tracks: Value) -> String {
    json!({
        "schema_version": 51,
        "name": "Before shapes",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": assets,
        "tracks": tracks,
    })
    .to_string()
}

fn migrated(json: &str) -> Ratio {
    let (project, from) = parse(json).expect("a v51 document migrates");
    assert_eq!(from, Some(51));
    project.validate().expect("and what comes out is valid");
    project.aspect
}

#[test]
fn a_vertical_edit_takes_the_shape_of_its_first_sized_clip() {
    let json = v51(
        json!([
            { "id": "title", "kind": "text", "text": "Hi" },
            { "id": "wide", "kind": "generated_video", "prompt": "a field", "state": "sketch",
              "video": { "aspect": "16:9" } },
            { "id": "phone", "kind": "video", "path": "assets/phone.mp4", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
              "media": { "width": 1080, "height": 1920 } }
        ]),
        json!([
            { "id": "v1", "kind": "video", "clips": [
                { "id": "later", "asset": "wide", "start": 30, "duration": 30 },
                { "id": "first", "asset": "phone", "start": 0, "duration": 30 }
            ]},
            { "id": "v2", "kind": "video", "clips": [
                { "id": "words", "asset": "title", "start": 0, "duration": 30 }
            ]}
        ]),
    );
    assert_eq!(migrated(&json), Ratio::TALL);
}

#[test]
fn a_generated_still_says_its_shape_and_a_title_says_none() {
    let json = v51(
        json!([
            { "id": "title", "kind": "text", "text": "Hi" },
            { "id": "square", "kind": "generated_image", "prompt": "a cup", "state": "sketch",
              "image": { "aspect": "1:1" } }
        ]),
        json!([{ "id": "v1", "kind": "video", "clips": [
            { "id": "words", "asset": "title", "start": 0, "duration": 30 },
            { "id": "cup", "asset": "square", "start": 30, "duration": 30 }
        ]}]),
    );
    assert_eq!(migrated(&json), Ratio::new(1, 1).expect("square"));
}

#[test]
fn an_edit_with_nothing_sized_is_landscape() {
    let json = v51(
        json!([{ "id": "title", "kind": "text", "text": "Hi" }]),
        json!([{ "id": "v1", "kind": "video", "clips": [
            { "id": "words", "asset": "title", "start": 0, "duration": 30 }
        ]}]),
    );
    assert_eq!(migrated(&json), Ratio::WIDE);
    assert_eq!(migrated(&v51(json!([]), json!([]))), Ratio::WIDE);
}
