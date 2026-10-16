use serde_json::json;

use super::*;
use crate::SCHEMA_VERSION;

/// A 16:9 project: a landscape shot fitted under a title, a portrait phone
/// clip fitted beside it, and a landscape shot already filling.
fn landscape() -> Project {
    let sha = "a".repeat(64);
    let media = |width: u32, height: u32| json!({ "width": width, "height": height });
    let document = json!({
        "schema_version": SCHEMA_VERSION,
        "name": "Wide",
        "timeline_fps": { "num": 30, "den": 1 },
        "aspect": "16:9",
        "assets": [
            { "id": "field", "kind": "video", "path": "assets/field.mp4", "sha256": sha,
              "media": media(1920, 1080) },
            { "id": "tv", "kind": "video", "path": "assets/tv.mp4", "sha256": sha,
              "media": media(1440, 1080) },
            { "id": "phone", "kind": "video", "path": "assets/phone.mp4", "sha256": sha,
              "media": media(1080, 1920) },
            { "id": "title", "kind": "text", "text": "Hello" }
        ],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "shot", "asset": "field", "start": 0, "duration": 30 },
            { "id": "upright", "asset": "phone", "start": 30, "duration": 30 },
            { "id": "covered", "asset": "field", "start": 60, "duration": 30, "fit": "fill" },
            { "id": "half", "asset": "field", "start": 90, "duration": 30,
              "crop": { "x": 0.0, "y": 0.0, "width": 0.5, "height": 1.0 } },
            { "id": "quarter", "asset": "field", "start": 120, "duration": 30,
              "crop": { "x": 0.25, "y": 0.25, "width": 0.5, "height": 0.5 } },
            { "id": "letter", "asset": "tv", "start": 150, "duration": 30,
              "crop": { "x": 0.0, "y": 0.125, "width": 1.0, "height": 0.75 } },
            { "id": "boxed", "asset": "tv", "start": 180, "duration": 30 }
        ]}, { "id": "v2", "kind": "video", "clips": [
            { "id": "words", "asset": "title", "start": 0, "duration": 30 }
        ]}]
    });
    Project::from_json(&document.to_string()).expect("a valid project")
}

fn fit_of(project: &Project, clip: &str) -> Fit {
    let (_, clip) = project
        .every_clip()
        .find(|(_, candidate)| candidate.id.as_str() == clip)
        .expect("the clip");
    clip.fit
}

#[test]
fn footage_that_filled_the_old_frame_fills_the_new_one() {
    let mut project = landscape();
    let refit = refit(&mut project, Ratio::TALL);
    assert!(refit.changed());
    assert_eq!(project.aspect, Ratio::TALL);
    let filled: Vec<&str> = refit.filled.iter().map(ClipId::as_str).collect();
    // A 16:9 crop filled it too, of 16:9 footage or of 4:3.
    assert_eq!(filled, ["shot", "quarter", "letter"]);
    assert_eq!(
        fit_of(&project, "boxed"),
        Fit::Fit,
        "4:3 whole never filled it"
    );
    assert_eq!(fit_of(&project, "shot"), Fit::Fill);
    // It never filled the landscape frame, so it is left as it was meant.
    assert_eq!(fit_of(&project, "upright"), Fit::Fit);
    assert_eq!(fit_of(&project, "half"), Fit::Fit, "its crop is 8:9");
    assert_eq!(fit_of(&project, "covered"), Fit::Fill);
    assert_eq!(fit_of(&project, "words"), Fit::Fit);
}

#[test]
fn the_same_shape_changes_nothing() {
    let mut project = landscape();
    let before = project.clone();
    let refit = refit(&mut project, Ratio::WIDE);
    assert!(!refit.changed());
    assert!(refit.filled.is_empty());
    assert_eq!(project, before);
}

#[test]
fn a_generated_shot_or_still_is_weighed_by_the_aspect_it_was_asked_for() {
    let document = json!({
        "schema_version": SCHEMA_VERSION,
        "name": "Tall",
        "timeline_fps": { "num": 30, "den": 1 },
        "aspect": "9:16",
        "assets": [{ "id": "shot", "kind": "generated_video", "prompt": "rain",
                     "state": "sketch", "video": { "aspect": "9:16" } },
                   { "id": "still", "kind": "generated_image", "prompt": "a door",
                     "state": "sketch", "image": { "aspect": "9:16" } }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "rain", "asset": "shot", "start": 0, "duration": 30 },
            { "id": "door", "asset": "still", "start": 30, "duration": 30 }
        ]}]
    });
    let mut project = Project::from_json(&document.to_string()).expect("a valid project");
    let refit = refit(&mut project, Ratio::new(1, 1).expect("square"));
    assert_eq!(refit.filled.len(), 2);
    assert_eq!(fit_of(&project, "rain"), Fit::Fill);
    assert_eq!(fit_of(&project, "door"), Fit::Fill);
}
