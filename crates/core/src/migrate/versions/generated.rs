//! The steps over generated assets — shots, stills and lines — held to the
//! same rule: a document at the old version, migrated, loads and validates at
//! this one, and asks the vendor for what it asked for before.

use serde_json::json;

use super::super::*;

/// v44 → v45: a v44 document's shots keep their tier and raster.
#[test]
fn a_v44_document_s_shots_keep_their_tier_and_raster() {
    let document = json!({
        "schema_version": 44,
        "name": "Before Standard and 4K",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "shot", "kind": "generated_video", "state": "sketch", "prompt": "rain",
              "video": { "model": "lite", "resolution": "720p", "seconds": 4 } }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v44 document migrates");
    assert_eq!(from, Some(44));
    project.validate().expect("and it validates");
    let request = project.assets[0].video_request();
    assert_eq!(request.model, crate::VideoModel::Lite);
    assert_eq!(request.resolution, crate::VideoResolution::P720);
}

/// v46 → v47: a v46 document's stills, sketched and generated, read the same.
#[test]
fn a_v46_document_s_stills_read_the_same_at_this_version() {
    let document = json!({
        "schema_version": 46,
        "name": "Before batches",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "poster", "kind": "generated_image", "state": "sketch", "prompt": "a poster",
              "image": { "model": "pro", "resolution": "2K" } }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v46 document migrates");
    assert_eq!(from, Some(46));
    project.validate().expect("and it validates");
    assert_eq!(project.assets[0].operation, None);
    assert_eq!(
        project.assets[0].image_request().model,
        crate::ImageModel::Pro
    );
}

/// v47 → v48: a v47 document's lines keep their model.
#[test]
fn a_v47_document_s_lines_keep_their_model() {
    let document = json!({
        "schema_version": 47,
        "name": "Before v4",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "vo", "kind": "generated_audio", "state": "sketch", "prompt": "hello",
              "speech": { "model": "expressive", "language": "pt" } }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v47 document migrates");
    assert_eq!(from, Some(47));
    project.validate().expect("and it validates");
    let request = project.assets[0].speech_request();
    assert_eq!(request.model, crate::SpeechModel::Expressive);
    assert_eq!(request.model.model_id(), "eleven_v3");
}
