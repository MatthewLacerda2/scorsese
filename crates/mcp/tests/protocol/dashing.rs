//! A dashed border authored over the wire, and a pattern that could not be one.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

fn shape_of(dir: &std::path::Path, id: &str) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk");
    let document: Value = serde_json::from_str(&text).expect("the project is JSON");
    document["assets"]
        .as_array()
        .expect("an assets table")
        .iter()
        .find(|asset| asset["id"] == id)
        .map(|asset| asset["shape"].clone())
        .expect("the shape was written")
}

#[test]
fn a_dash_pattern_lands_on_the_shape_as_written() {
    let dir = project("shape-dash");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "geometry": "arrow", "stroke": "#ffffff",
                "from": { "x": 0.1, "y": 0.5 }, "to": { "x": 0.9, "y": 0.5 },
                "dash": [0.02, 0.012], "asset": "flow" }),
    ));
    assert!(!failed, "{text}");
    assert_eq!(shape_of(&dir, "flow")["dash"], json!([0.02, 0.012]));
    std::fs::remove_dir_all(dir).ok();
}

/// An empty pattern is refused by validation, in the words a hand-written
/// document gets; something that is not a list of numbers never gets that far.
#[test]
fn a_pattern_that_is_no_pattern_is_refused() {
    for (label, dash) in [("empty", json!([])), ("words", json!(["long", "short"]))] {
        let dir = project(&format!("shape-dash-{label}"));
        let (text, failed) = said(&call(
            "asset_set",
            json!({ "project": dir, "kind": "shape", "geometry": "ellipse", "width": 0.2, "height": 0.2,
                    "stroke": "#ffffff", "dash": dash }),
        ));
        assert!(failed, "{label}: {text}");
        assert!(
            text.contains("dash"),
            "{label}: says what was wrong: {text}"
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
