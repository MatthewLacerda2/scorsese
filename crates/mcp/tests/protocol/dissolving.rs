//! Dissolving over a clip that already fades: a clip carries one opacity
//! track (#1099), so the dissolve's ramp replaces it, and the reply says so.

use super::fixture::{project, read};
use crate::{call, said};
use serde_json::json;

#[test]
fn a_dissolve_names_the_fade_it_replaced() {
    let dir = project("dissolve-faded");
    let mut doc = read(&dir);
    doc["tracks"][0]["clips"] = json!([
        { "id": "c1", "asset": "title", "start": 0, "duration": 300,
          "keyframes": [ { "property": "opacity",
            "keyframes": [ { "t": 0, "value": 0.0 }, { "t": 30, "value": 1.0 } ] } ] },
        { "id": "c2", "asset": "title", "start": 300, "duration": 300 }
    ]);
    std::fs::write(dir.join("project.json"), doc.to_string()).expect("write");

    let (text, failed) = said(&call(
        "dissolve",
        json!({ "project": dir, "from": "c1", "to": "c2" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("replaced the opacity keyframes already on `c1` (2 point(s))"),
        "got {text}"
    );
    assert!(!text.contains("`c2` ("), "c2 had no fade: {text}");

    let keys = &read(&dir)["tracks"][0]["clips"][0]["keyframes"];
    assert_eq!(keys.as_array().map(Vec::len), Some(1), "one opacity track");
    assert_eq!(keys[0]["by"], "dissolve");
    std::fs::remove_dir_all(dir).ok();
}
