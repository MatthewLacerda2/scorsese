use serde_json::{Value, json};

use crate::migrate::parse;

fn track(property: &str, value: f64, by: Option<&str>) -> Value {
    let mut track = json!({ "property": property, "keyframes": [{ "t": 0, "value": value }] });
    if let Some(by) = by {
        track["by"] = json!(by);
    }
    track
}

fn v54(clip_keyframes: Value, member_keyframes: Value) -> Value {
    json!({
        "schema_version": 54,
        "name": "Two tracks on one property",
        "timeline_fps": { "num": 30, "den": 1 },
        "aspect": "16:9",
        "assets": [
            { "id": "bed", "kind": "color", "color": "#101820" },
            { "id": "song", "kind": "audio", "path": "assets/song.wav" },
            { "id": "card", "kind": "group", "group": { "tracks": [
                { "id": "g1", "kind": "video", "clips": [
                    { "id": "inner", "asset": "bed", "start": 0, "duration": 30,
                      "keyframes": member_keyframes }
                ]}
            ]}}
        ],
        "tracks": [
            { "id": "v1", "kind": "video", "clips": [
                { "id": "c1", "asset": "bed", "start": 0, "duration": 30,
                  "keyframes": clip_keyframes },
                { "id": "c2", "asset": "card", "start": 30, "duration": 30 }
            ]},
            { "id": "a1", "kind": "audio", "clips": [
                { "id": "m1", "asset": "song", "start": 0, "duration": 30, "keyframes": [
                    track("volume", 0.5, None), track("volume", 0.25, Some("duck"))
                ]}
            ]}
        ]
    })
}

/// The track kept is the one the compositor drew at v54 — the last — and every
/// other property, and every volume track, is left as it was.
#[test]
fn a_v54_clip_keeps_the_last_track_on_each_property() {
    let document = v54(
        json!([
            track("opacity", 0.2, None),
            track("transform.scale.x", 2.0, None),
            track("opacity", 0.4, Some("dissolve")),
            track("opacity", 0.6, None),
        ]),
        json!([track("blur", 0.1, None), track("blur", 0.3, None)]),
    );
    let (project, from) = parse(&document.to_string()).expect("a v54 document migrates");
    assert_eq!(from, Some(54));
    project.validate().expect("and it validates");

    let read = |clip: &str| -> Vec<(String, f64)> {
        let (_, clip) = project
            .every_clip()
            .find(|(_, found)| found.id.as_str() == clip)
            .expect("the clip");
        clip.keyframes
            .iter()
            .map(|track| (track.property.to_string(), track.keyframes[0].value))
            .collect()
    };
    let named = |pairs: &[(&str, f64)]| -> Vec<(String, f64)> {
        pairs
            .iter()
            .map(|(path, value)| (path.to_string(), *value))
            .collect()
    };
    assert_eq!(
        read("c1"),
        named(&[("transform.scale.x", 2.0), ("opacity", 0.6)])
    );
    assert_eq!(
        read("inner"),
        named(&[("blur", 0.3)]),
        "a group's member too"
    );
    assert_eq!(read("m1"), named(&[("volume", 0.5), ("volume", 0.25)]));
}
