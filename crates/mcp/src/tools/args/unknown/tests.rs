//! Which name a typo was meant to be, and the walk into nested objects.

use serde_json::json;

use super::{nearest, refused};

const KNOWN: &[&str] = &[
    "clip",
    "scale",
    "start_seconds",
    "duration_seconds",
    "speed",
];

#[test]
fn a_typo_or_a_missing_unit_is_named() {
    assert_eq!(nearest("sclae", KNOWN), Some("scale"));
    assert_eq!(nearest("duration", KNOWN), Some("duration_seconds"));
    assert_eq!(nearest("start", KNOWN), Some("start_seconds"));
    // One letter off a three-letter name is still that name.
    assert_eq!(nearest("fot", &["fit"]), Some("fit"));
}

#[test]
fn nothing_is_named_unless_one_answer_stands_out() {
    assert_eq!(nearest("opacity", KNOWN), None);
    // As close to one as to the other: no guess.
    assert_eq!(nearest("cot", &["cat", "cut"]), None);
    // Two letters off a four-letter name is a different word.
    assert_eq!(nearest("clop", &["chip"]), None);
}

/// A schema with every shape the walk goes into.
fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "clip": { "type": "string" },
            "shadow": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "softness": {}, "opacity": {} }
            },
            "scenes": {
                "type": "array",
                "items": { "type": "object", "properties": { "line": {}, "visuals": {} } }
            },
            "fill": { "type": ["string", "object"] },
            "open": { "type": "object", "properties": { "a": {} }, "additionalProperties": true },
            "either": {
                "anyOf": [
                    { "type": "object", "properties": { "x": {} } },
                    { "type": "object", "properties": { "y": {} } }
                ]
            }
        }
    })
}

#[test]
fn a_misspelt_key_inside_an_object_is_refused_by_its_path() {
    assert_eq!(
        refused(&json!({ "shadow": { "softnes": 4 } }), &schema()).as_deref(),
        Some(
            "`shadow.softnes` is not an argument here; did you mean `shadow.softness`? \
             Nothing was done; `shadow` takes `opacity`, `softness`."
        )
    );
    let refused = refused(
        &json!({ "scenes": [{ "line": "a" }, { "line": "b", "visual": [] }] }),
        &schema(),
    );
    assert!(
        refused.as_deref().is_some_and(|said| said.starts_with(
            "`scenes[1].visual` is not an argument here; did you mean `scenes[1].visuals`?"
        )),
        "{refused:?}"
    );
}

#[test]
fn the_top_level_is_named_first_and_without_a_path() {
    let refused = refused(&json!({ "clp": "c", "shadow": { "x": 1 } }), &schema()).unwrap();
    assert!(refused.starts_with("`clp` is not an argument here; did you mean `clip`?"));
    assert!(refused.ends_with("it takes `clip`, `either`, `fill`, `open`, `scenes`, `shadow`."));
}

#[test]
fn what_lists_no_keys_or_takes_any_stays_open() {
    let open = json!({
        "clip": "c",
        "shadow": false,
        "fill": { "stops": [] },
        "open": { "anything": 1 },
        "either": { "y": 2 }
    });
    assert_eq!(refused(&open, &schema()), None);
    // A choice refuses only when no branch takes the value.
    let refused = refused(&json!({ "either": { "z": 2 } }), &schema());
    assert!(refused.is_some_and(|said| said.starts_with("`either.z`")));
}
