//! The steps held to their rule — a document at the old version, migrated,
//! loads and validates at this one — and the walk that chains them.

use serde_json::json;

use super::*;

#[test]
fn every_version_since_the_oldest_has_exactly_one_step() {
    // The rule, held: bump SCHEMA_VERSION without a step and this fails.
    let froms: Vec<u32> = STEPS.iter().map(|step| step.from).collect();
    let expected: Vec<u32> = (OLDEST_MIGRATABLE..SCHEMA_VERSION).collect();
    assert_eq!(froms, expected, "one step per version, oldest first");
}

fn mark(document: &mut Value, marker: &str) {
    let trail = document["trail"].as_array_mut().expect("a trail");
    trail.push(Value::from(marker));
}

const CHAIN: &[Step] = &[
    Step {
        from: 1,
        apply: |document| {
            mark(document, "1→2");
            Ok(())
        },
    },
    Step {
        from: 2,
        apply: |document| {
            mark(document, "2→3");
            Ok(())
        },
    },
];

#[test]
fn the_chain_runs_every_step_in_order_and_sets_the_version() {
    let mut document = json!({ "schema_version": 1, "trail": [] });
    assert_eq!(walk(&mut document, CHAIN, 3).unwrap(), Some(1));
    assert_eq!(
        document,
        json!({ "schema_version": 3, "trail": ["1→2", "2→3"] })
    );

    let mut halfway = json!({ "schema_version": 2, "trail": [] });
    assert_eq!(walk(&mut halfway, CHAIN, 3).unwrap(), Some(2));
    assert_eq!(halfway["trail"], json!(["2→3"]));
}

#[test]
fn a_current_document_is_left_exactly_as_it_was() {
    let mut document = json!({ "schema_version": 3, "trail": [] });
    assert_eq!(walk(&mut document, CHAIN, 3).unwrap(), None);
    assert_eq!(document["trail"], json!([]));
}

#[test]
fn nothing_goes_backwards_and_nothing_is_guessed() {
    let mut newer = json!({ "schema_version": 4, "trail": [] });
    assert!(matches!(
        walk(&mut newer, CHAIN, 3),
        Err(MigrateError::Newer {
            found: 4,
            supported: 3
        })
    ));
    let mut ancient = json!({ "schema_version": 0, "trail": [] });
    assert!(matches!(
        walk(&mut ancient, CHAIN, 3),
        Err(MigrateError::TooOld { found: 0 })
    ));
    let mut unversioned = json!({ "name": "x" });
    assert!(matches!(
        walk(&mut unversioned, CHAIN, 3),
        Err(MigrateError::Unversioned)
    ));
}

/// The real step, held to the rule every step is held to: a document at
/// the old version, migrated, loads and validates at this one. A v33
/// document with something of every common shape in it — an imported
/// shot, a title, a colour, keyframes — is the realistic case.
#[test]
fn a_v33_document_walks_to_this_version_and_validates() {
    let document = json!({
        "schema_version": 33,
        "name": "Before groups",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "bed", "kind": "color", "color": "#101820" },
            { "id": "title", "kind": "text", "text": "Hello" }
        ],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-bed", "asset": "bed", "start": 0, "duration": 30 }
        ]}, { "id": "v2", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 0, "duration": 30,
              "keyframes": [{ "property": "opacity", "keyframes": [
                  { "t": 0, "value": 0.0 }, { "t": 10, "value": 1.0 }] }] }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v33 document migrates");
    assert_eq!(from, Some(33));
    assert_eq!(project.schema_version, SCHEMA_VERSION);
    project
        .validate()
        .expect("and what comes out is a valid document");
    assert_eq!(project.tracks.len(), 2, "nothing was dropped on the way");
}

/// v34 → v35 held to the same rule, over the part of a document the
/// version touched: every easing a v34 keyframe could name still names the
/// same curve after the walk.
#[test]
fn a_v34_documents_easings_read_the_same_at_this_version() {
    use crate::Easing;
    let words = ["linear", "ease_in", "ease_out", "ease_in_out", "hold"];
    let keyframes: Vec<Value> = (0u64..)
        .zip(words)
        .map(|(t, easing)| json!({ "t": t * 10, "value": 0.5, "easing": easing }))
        .collect();
    let document = json!({
        "schema_version": 34,
        "name": "Before overshoot",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "title", "kind": "text", "text": "Hello" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 0, "duration": 60,
              "keyframes": [{ "property": "opacity", "keyframes": keyframes }] }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v34 document migrates");
    assert_eq!(from, Some(34));
    project.validate().expect("and it validates");
    let read: Vec<Easing> = project.tracks[0].clips[0].keyframes[0]
        .keyframes
        .iter()
        .map(|keyframe| keyframe.easing)
        .collect();
    let expected = [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::Hold,
    ];
    assert_eq!(read, expected);
}

/// v35 → v36 over a document with a shape in it — the kind the version
/// touched — keyframed on a path the new build now animates.
#[test]
fn a_v35_document_with_a_shape_walks_to_this_version_and_validates() {
    let document = json!({
        "schema_version": 35,
        "name": "Before dashes",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "box", "kind": "shape", "shape": {
            "geometry": { "rectangle": { "width": 0.3, "height": 0.2 } },
            "stroke": "#ffffffff" } }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-box", "asset": "box", "start": 0, "duration": 30 }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v35 document migrates");
    assert_eq!(from, Some(35));
    assert_eq!(project.schema_version, SCHEMA_VERSION);
    project.validate().expect("and it is valid");
    let shape = project.assets[0].shape.as_ref().expect("still a shape");
    assert_eq!(shape.dash, None, "and still solid");
}

/// v36 → v37: a v36 caption that happens to say `{n}` still says it — the
/// braces became a placeholder only for a text that has a `number` block.
#[test]
fn a_v36_caption_reads_the_same_at_this_version() {
    let document = json!({
        "schema_version": 36,
        "name": "Before counting",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "title", "kind": "text", "text": "{n} is a variable" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 0, "duration": 30 }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v36 document migrates");
    assert_eq!(from, Some(36));
    project.validate().expect("and it validates");
    let title = &project.assets[0];
    assert_eq!(title.text.as_deref(), Some("{n} is a variable"));
    assert_eq!(title.text_style().number, None);
    assert_eq!(title.text_style().reveal, None);
}

#[test]
fn a_refusing_step_names_itself() {
    const REFUSES: &[Step] = &[Step {
        from: 1,
        apply: |_| Err("no new equivalent".to_owned()),
    }];
    let mut document = json!({ "schema_version": 1 });
    let error = walk(&mut document, REFUSES, 2).unwrap_err();
    assert!(
        matches!(error, MigrateError::Failed { from: 1, .. }),
        "{error}"
    );
}
