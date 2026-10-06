//! Gradient fills authored over the wire: a shape's `fill` and a colour
//! asset's `color` take the object form `project.json` writes, and what a
//! gradient cannot be is refused in validation's words.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

fn asset_of(dir: &std::path::Path, id: &str) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk");
    let document: Value = serde_json::from_str(&text).expect("the project is JSON");
    document["assets"]
        .as_array()
        .expect("an assets table")
        .iter()
        .find(|asset| asset["id"] == id)
        .cloned()
        .expect("the asset was written")
}

#[test]
fn a_gradient_pill_and_a_gradient_backdrop_land_as_written() {
    let dir = project("gradient-new");
    let linear = json!({ "linear": { "angle": 90.0,
        "stops": [["#e8590c", 0.0], ["#7048e8", 1.0]] } });
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "shape", "geometry": "rectangle", "width": 0.6, "height": 0.2,
                "radius": 0.5, "fill": linear, "asset": "plate" }),
    ));
    assert!(!failed, "{text}");
    assert_eq!(asset_of(&dir, "plate")["shape"]["fill"], linear);

    let radial = json!({ "radial": { "center": { "x": 0.5, "y": 0.45 }, "radius": 0.8,
        "stops": [["#1b2440", 0.0], ["#0b1020", 1.0]] } });
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "color", "color": radial, "asset": "backdrop" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("radial gradient"),
        "says what it wrote: {text}"
    );
    assert_eq!(asset_of(&dir, "backdrop")["color"], radial);
    std::fs::remove_dir_all(dir).ok();
}

/// One stop is a colour written the long way; validation says so, and
/// nothing is written.
#[test]
fn a_gradient_with_one_stop_is_refused() {
    let dir = project("gradient-one-stop");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "color", "asset": "backdrop",
                "color": { "linear": { "stops": [["#000000", 0.0]] } } }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("at least two"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A caption is one colour. `asset_set` refuses a gradient on it by name.
#[test]
fn a_caption_refuses_a_gradient_colour() {
    let dir = project("gradient-caption");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "kind": "text", "text": "Hello", "asset": "caption" }),
    ));
    assert!(!failed, "{text}");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "caption",
                "color": { "linear": { "stops": [["#000000", 0], ["#ffffff", 1]] } } }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("one colour"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
