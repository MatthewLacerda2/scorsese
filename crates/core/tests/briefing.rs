//! The rest of a generated video's brief, and the stamps beside it: how they
//! are spelled in the document, and what the format refuses to store.

mod common;

use common::document;
use scorsese_core::{
    Aspect, ClipSeconds, LoadError, Project, Timestamp, VideoModel, VideoResolution,
};

/// A project whose one asset is a Veo sketch carrying `video`.
fn with_request(request: &str) -> Result<Project, LoadError> {
    Project::from_json(&document(&format!(
        r#""assets": [{{ "id": "shot", "kind": "generated_video", "state": "sketch",
                        "prompt": "rain on a window", "video": {request} }}]"#
    )))
}

#[test]
fn a_request_is_spelled_the_way_the_page_says() {
    let project = with_request(
        r#"{ "model": "lite", "resolution": "720p", "seconds": 6, "aspect": "9:16" }"#,
    )
    .expect("that is the documented spelling");
    let request = project.assets[0].video_request();
    assert_eq!(request.model, VideoModel::Lite);
    assert_eq!(request.resolution, VideoResolution::P720);
    assert_eq!(request.seconds, ClipSeconds::Six);
    assert_eq!(request.aspect, Aspect::Tall);
}

/// An absent request is every default rather than an absence — which is what
/// lets the shortest useful brief be a sentence and nothing else.
#[test]
fn saying_nothing_asks_for_the_defaults() {
    let project = Project::from_json(&document(
        r#""assets": [{ "id": "shot", "kind": "generated_video", "state": "sketch",
                       "prompt": "rain on a window" }]"#,
    ))
    .expect("a prompt alone is a complete brief");
    let asset = &project.assets[0];
    assert_eq!(asset.video, None, "nothing was written, so nothing is read");
    assert_eq!(asset.video_request().seconds, ClipSeconds::Eight);
    assert_eq!(asset.video_request().resolution, VideoResolution::P1080);
    assert!(project.is_valid());
}

#[test]
fn a_length_the_provider_does_not_generate_is_not_storable() {
    let refused = with_request(r#"{ "seconds": 5 }"#).expect_err("5 is not one of the three");
    assert!(
        refused.to_string().contains("4, 6 or 8"),
        "the refusal should say what the three are: {refused}"
    );
}

#[test]
fn a_misspelled_field_is_refused_rather_than_dropped() {
    with_request(r#"{ "modell": "fast" }"#)
        .expect_err("a typo in a brief must fail the load, not be silently ignored");
}

#[test]
fn a_request_survives_a_round_trip() {
    let project = with_request(
        r#"{ "model": "fast", "resolution": "720p", "seconds": 4, "aspect": "16:9",
             "first_image": "doorway", "reference_images": ["face"] }"#,
    )
    .expect("a full request parses");
    let reparsed = Project::from_json(&project.to_json().expect("serialise")).expect("reparse");
    assert_eq!(project, reparsed);
}

#[test]
fn an_instant_is_utc_whole_seconds_and_nothing_else() {
    Timestamp::try_from("2026-08-04T14:20:00Z".to_owned()).expect("the one accepted spelling");
    for rejected in [
        "2026-08-04T14:20:00+01:00",
        "2026-08-04T14:20:00.500Z",
        "2026-08-04 14:20:00Z",
        "2026-13-04T14:20:00Z",
        "2026-08-04T25:20:00Z",
        "",
    ] {
        Timestamp::try_from(rejected.to_owned()).expect_err(&format!(
            "`{rejected}` is not an instant this format stores"
        ));
    }
}

/// The reason the spelling is pinned: one format, whole seconds and UTC means
/// sorting these as text is sorting them as times, so a pool too long to read
/// can be ordered without parsing anything.
#[test]
fn instants_sort_as_text_the_way_they_sort_as_time() {
    let earlier = Timestamp::from_utc(2026, 8, 4, 9, 0, 0).expect("a real instant");
    let later = Timestamp::from_utc(2026, 8, 4, 14, 20, 0).expect("a real instant");
    assert!(earlier < later);
    assert!(earlier.as_str() < later.as_str());
}

#[test]
fn a_stamp_survives_a_round_trip_and_is_never_nulled() {
    let project = Project::from_json(&document(
        r#""assets": [{ "id": "logo", "kind": "image", "path": "assets/logo.png",
                       "created_at": "2026-08-04T14:20:00Z" }]"#,
    ))
    .expect("a stamped asset parses");
    let json = project.to_json().expect("serialise");
    assert!(json.contains("2026-08-04T14:20:00Z"));
    assert!(!json.contains("null"));
    assert_eq!(project, Project::from_json(&json).expect("reparse"));
}
