//! Where a song's sections are, said precisely enough to place a clip on —
//! whether or not the bake that says it rendered anything.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// The starter song is four patterns of four beats at 96 bpm: a section every
/// two and a half seconds. The report says so to the millisecond, fresh and
/// cached alike — a caption is put on a section long after the bake that
/// measured it, and the bounds are the recipe's arithmetic, not a measurement.
#[test]
fn a_bake_says_where_each_section_is_even_when_it_was_already_baked() {
    let dir = project("sections");
    let (text, failed) = said(&call(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    ));
    assert!(!failed, "{text}");

    for (pass, expected) in [(1, "theme — baked"), (2, "theme — already baked")] {
        let (text, failed) = said(&call("synth_bake", json!({ "project": dir })));
        assert!(!failed, "{text}");
        assert!(text.contains(expected), "bake {pass}: {text}");
        for bounds in ["0.000-2.500", "2.500-5.000", "7.500-10.000"] {
            assert!(text.contains(bounds), "bake {pass} lacks {bounds}: {text}");
        }
    }
    std::fs::remove_dir_all(dir).ok();
}
