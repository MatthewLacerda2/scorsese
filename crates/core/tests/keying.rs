//! What a document says about a chroma key, and what it says by leaving one
//! out.
//!
//! The key is the first picture property on a clip that is an `Option` of a
//! struct rather than a number with a neutral, which is what these are about:
//! absence has to mean *no key*, and a key that is present has to survive a
//! save with every value somebody chose still on it.

mod common;

use scorsese_core::{ChromaKey, Project, Rgba};

/// The green most screens are painted.
const SCREEN: Rgba = Rgba::opaque(0, 177, 64);

/// The fixture's `c-logo` — an `image`, so a key on it is legal — carrying
/// `key`.
fn with(key: Option<ChromaKey>) -> Project {
    let mut project = common::project();
    project.tracks[1].clips[0].chroma_key = key;
    project
}

/// Almost every clip has no key, so almost every clip must write nothing about
/// one — which is what makes a load-and-save a fixed point rather than a slow
/// accretion of defaults.
#[test]
fn a_clip_with_no_key_writes_nothing_at_all() {
    let json = common::project().to_json().expect("serialise");
    assert!(!json.contains("chroma_key"), "{json}");
}

/// And the other half, which a predicate that skipped everything would also
/// satisfy: all four values somebody chose come back.
#[test]
fn a_key_somebody_wrote_survives_the_save() {
    let project = with(Some(ChromaKey {
        color: SCREEN,
        tolerance: 0.31,
        softness: 0.07,
        spill: true,
    }));
    let json = project.to_json().expect("serialise");
    assert!(json.contains("\"color\": \"#00b140\""), "{json}");
    assert!(json.contains("\"tolerance\": 0.31"), "{json}");
    assert!(json.contains("\"softness\": 0.07"), "{json}");
    assert!(json.contains("\"spill\": true"), "{json}");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
    assert_eq!(project.validate(), Ok(()));
}

/// A key with nothing on it but a screen colour is the shortest thing anybody
/// would write, and it has to *key* rather than do nothing — so the two
/// numbers default to values that work.
#[test]
fn a_key_with_only_a_colour_takes_the_defaults() {
    let json = common::document(
        r##""assets": [{ "id": "a", "kind": "image", "path": "assets/a.png" }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c", "asset": "a", "start": 0, "duration": 30,
                 "chroma_key": { "color": "#00b140" } }] }]"##,
    );
    let project = Project::from_json(&json).expect("parses");
    let (_, clip) = project.clips().next().expect("a clip");
    let key = clip.chroma_key.expect("a key");
    assert_eq!(key.color, SCREEN);
    assert!((key.tolerance - ChromaKey::TOLERANCE).abs() < f64::EPSILON);
    assert!((key.softness - ChromaKey::SOFTNESS).abs() < f64::EPSILON);
    assert!(!key.spill, "the suppression is off unless it is asked for");
    assert!(key.tolerance > 0.0, "and the defaults key something");
}

/// An absent key is no key, not a key at its defaults — decided here so nothing
/// downstream has to ask what a missing field means.
#[test]
fn an_absent_key_is_no_key_at_all() {
    let json = common::document(
        r#""assets": [{ "id": "a", "kind": "image", "path": "assets/a.png" }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c", "asset": "a", "start": 0, "duration": 30 }] }]"#,
    );
    let project = Project::from_json(&json).expect("parses");
    let (_, clip) = project.clips().next().expect("a clip");
    assert_eq!(clip.chroma_key, None);
}

/// A key needs a screen colour, and a misspelled field is refused rather than
/// silently ignored — the difference between a document that keys nothing and
/// one that says so.
#[test]
fn a_key_without_a_colour_or_with_a_field_nobody_knows_is_refused() {
    for key in [
        r##"{ "tolerance": 0.3 }"##,
        r##"{ "color": "#00b140", "tolerence": 0.3 }"##,
        r##"{ "color": "not a colour" }"##,
    ] {
        let json = common::document(&format!(
            r#""assets": [{{ "id": "a", "kind": "image", "path": "assets/a.png" }}],
               "tracks": [{{ "id": "v1", "kind": "video", "clips": [
                   {{ "id": "c", "asset": "a", "start": 0, "duration": 30,
                     "chroma_key": {key} }}] }}]"#
        ));
        assert!(Project::from_json(&json).is_err(), "{key} was accepted");
    }
}
