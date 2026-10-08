//! Each real step, held to the rule every step is held to: a document at the
//! old version, migrated, loads and validates at this one — over the part of
//! the document that version touched.

use serde_json::json;

use super::*;

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

/// v37 → v38: a v37 clip — which cannot say `shadow`, `glow` or `blend` —
/// comes out with none of the three, the source-over, unlit layer it always was.
#[test]
fn a_v37_clip_arrives_unlit_and_normal() {
    let document = json!({
        "schema_version": 37,
        "name": "Before light",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "title", "kind": "text", "text": "Hello" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 0, "duration": 60, "blur": 0.01 }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v37 document migrates");
    assert_eq!(from, Some(37));
    project.validate().expect("and it validates");
    let clip = &project.tracks[0].clips[0];
    assert_eq!((clip.shadow, clip.glow), (None, None));
    assert_eq!(clip.blend, crate::Blend::Normal);
}

/// v38 → v39: a v38 document with an arrow and a clip beside it — the pair
/// the version lets one follow the other — comes out with nothing following.
#[test]
fn a_v38_document_with_an_arrow_walks_to_this_version_and_validates() {
    let document = json!({
        "schema_version": 38,
        "name": "Before following",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "line", "kind": "shape", "shape": {
                "geometry": { "arrow": { "from": { "x": 0.1, "y": 0.5 },
                                         "to": { "x": 0.9, "y": 0.5 } } },
                "stroke": "#ffffffff" } },
            { "id": "dot", "kind": "shape", "shape": {
                "geometry": { "ellipse": { "width": 0.05, "height": 0.09 } },
                "fill": "#ffcc00ff" } }
        ],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-line", "asset": "line", "start": 0, "duration": 30 }
        ]}, { "id": "v2", "kind": "video", "clips": [
            { "id": "c-dot", "asset": "dot", "start": 0, "duration": 30 }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v38 document migrates");
    assert_eq!(from, Some(38));
    project.validate().expect("and it is valid");
    let followers = project.every_clip().filter(|(_, c)| c.follow.is_some());
    assert_eq!(followers.count(), 0, "and nothing follows anything");
}

/// v39 → v40 over the two fields the version widened: a colour string on
/// a shape's `fill` and on a colour asset still reads as that one colour.
#[test]
fn a_v39_documents_colours_read_the_same_at_this_version() {
    use crate::{Fill, Rgba};
    let document = json!({
        "schema_version": 39,
        "name": "Before gradients",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "bed", "kind": "color", "color": "#101820" },
            { "id": "box", "kind": "shape", "shape": {
                "geometry": { "ellipse": { "width": 0.3, "height": 0.2 } },
                "fill": "#1e3a8aff" } }
        ],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-bed", "asset": "bed", "start": 0, "duration": 30 }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v39 document migrates");
    assert_eq!(from, Some(39));
    project.validate().expect("and it is valid");
    let solid = |fill: Option<&Fill>| fill.and_then(Fill::solid);
    assert_eq!(
        solid(project.assets[0].color.as_ref()),
        Some(Rgba::opaque(0x10, 0x18, 0x20))
    );
    let shape = project.assets[1].shape.as_ref().expect("still a shape");
    assert_eq!(
        solid(shape.fill.as_ref()),
        Some(Rgba::opaque(0x1e, 0x3a, 0x8a))
    );
}

/// v40 → v41: a v40 clip — which cannot say `matte` — is shown whole, and
/// every clip it had is still drawn.
#[test]
fn a_v40_clip_arrives_unmatted() {
    let document = json!({
        "schema_version": 40,
        "name": "Before mattes",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "title", "kind": "text", "text": "Hello" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 0, "duration": 60, "blend": "add" }
        ]}]
    });
    let (project, from) = parse(&document.to_string()).expect("a v40 document migrates");
    assert_eq!(from, Some(40));
    project.validate().expect("and it validates");
    assert_eq!(project.tracks[0].clips[0].matte, None);
}

/// v41 → v42: a v41 shot built from an imported still keeps its still, and
/// the still stays an `image`.
#[test]
fn a_v41_shot_and_its_still_read_the_same_at_this_version() {
    let document = json!({
        "schema_version": 41,
        "name": "Before generated stills",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "door", "kind": "image", "path": "assets/door.png" },
            { "id": "shot", "kind": "generated_video", "state": "sketch",
              "prompt": "the door opens", "video": { "first_image": "door" } }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v41 document migrates");
    assert_eq!(from, Some(41));
    project.validate().expect("and it validates");
    assert_eq!(project.assets[0].kind, crate::AssetKind::Image);
    let first = project.assets[1].video_request().first_image;
    assert_eq!(
        first.map(|id| id.as_str().to_owned()),
        Some("door".to_owned())
    );
}

/// v42 → v43: a v42 document with stills in it reads the same, and the stills
/// stay `image` assets.
#[test]
fn a_v42_document_s_stills_read_the_same_at_this_version() {
    let document = json!({
        "schema_version": 42,
        "name": "Before sequences",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "f1", "kind": "image", "path": "assets/spin/0001.png" },
            { "id": "f2", "kind": "image", "path": "assets/spin/0002.png" }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v42 document migrates");
    assert_eq!(from, Some(42));
    project.validate().expect("and it validates");
    assert!(
        project
            .assets
            .iter()
            .all(|asset| asset.kind == crate::AssetKind::Image)
    );
}

/// v43 → v44: a v43 document's file-backed assets read the same, and none of
/// them became a page.
#[test]
fn a_v43_document_s_assets_read_the_same_at_this_version() {
    let document = json!({
        "schema_version": 43,
        "name": "Before pages",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "logo", "kind": "image", "path": "assets/logo.png" },
            { "id": "title", "kind": "text", "text": "Hello" }
        ],
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v43 document migrates");
    assert_eq!(from, Some(43));
    project.validate().expect("and it validates");
    assert_eq!(project.assets[0].kind, crate::AssetKind::Image);
    assert_eq!(project.assets[1].kind, crate::AssetKind::Text);
}

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
