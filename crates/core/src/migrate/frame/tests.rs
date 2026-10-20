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

/// One video track whose only clip shows `asset`.
fn showing(asset: Value) -> String {
    v51(
        json!([asset]),
        json!([{ "id": "v1", "kind": "video", "clips": [
            { "id": "first", "asset": "picture", "start": 0, "duration": 30 }
        ]}]),
    )
}

fn photo(width: u32, height: u32) -> Value {
    json!({ "id": "picture", "kind": "image", "path": "assets/photo.png",
            "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "media": { "width": width, "height": height } })
}

#[test]
fn a_picture_of_no_picked_shape_is_landscape() {
    // #1430: a 735x478 image framed a hosted project as 735:478.
    assert_eq!(migrated(&showing(photo(735, 478))), Ratio::WIDE);
    // A photograph's 3:2 says what camera took it, not what the edit is for.
    assert_eq!(migrated(&showing(photo(3000, 2000))), Ratio::WIDE);
    // 1000x1180 is 5.9% off 4:5, just past a twentieth.
    assert_eq!(migrated(&showing(photo(1000, 1180))), Ratio::WIDE);
    let asked = json!({ "id": "picture", "kind": "generated_image", "prompt": "a cup",
                        "state": "sketch", "image": { "aspect": "2:3" } });
    assert_eq!(migrated(&showing(asked)), Ratio::WIDE);
}

#[test]
fn a_picture_near_a_picked_shape_takes_it() {
    // 1080x1350 is 4:5 exactly; 1000x1280 is 2.3% off it, past the 1% a
    // size snaps by, and within a twentieth.
    let portrait = Ratio::new(4, 5).expect("4:5");
    assert_eq!(migrated(&showing(photo(1080, 1350))), portrait);
    assert_eq!(migrated(&showing(photo(1000, 1280))), portrait);
    // 1000x1300 is within a twentieth of 4:5 (3.8%) and of 3:4 (2.6%): the
    // nearer wins, whatever order the shapes are listed in.
    assert_eq!(
        migrated(&showing(photo(1000, 1300))),
        Ratio::new(3, 4).expect("3:4")
    );
    assert_eq!(
        migrated(&showing(photo(1440, 1080))),
        Ratio::new(4, 3).expect("4:3")
    );
    assert_eq!(
        migrated(&showing(photo(2560, 1080))),
        Ratio::new(21, 9).expect("21:9")
    );
    let shot = json!({ "id": "picture", "kind": "generated_video", "prompt": "a street",
                       "state": "sketch", "video": { "aspect": "9:16" } });
    assert_eq!(migrated(&showing(shot)), Ratio::TALL);
}
