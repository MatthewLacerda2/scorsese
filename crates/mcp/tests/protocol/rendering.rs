//! Asking for part of the timeline rather than all of it.
//!
//! Every render here is a handful of frames at a postage-stamp raster, which is
//! the point of the argument being tested: the whole fixture is twenty seconds
//! long and none of these pays for it.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// Small, because these encode for real.
const SMALL: &str = "160x90";

/// Renders `range` over the fixture and returns what the server said.
fn rendered(label: &str, range: Option<&str>) -> String {
    let dir = project(label);
    let mut arguments = json!({
        "project": dir, "out": dir.join("cut.mp4"), "resolution": SMALL
    });
    if let Some(range) = range {
        arguments["range"] = json!(range);
    }
    let (text, failed) = said(&call("render", arguments));
    assert!(!failed, "render refused: {text}");
    std::fs::remove_dir_all(dir).ok();
    text
}

/// The closed form: exactly the frames asked for reach the file.
#[test]
fn a_frame_range_encodes_only_that_stretch() {
    let text = rendered("render-range", Some("10:13"));
    assert!(text.contains("3 frames"), "got {text}");
}

/// The open form the CLI accepts, and the one most easily dropped by a second
/// parser: `598:` runs to whatever the timeline turns out to be, which only the
/// plan knows.
#[test]
fn an_open_ended_range_runs_to_the_end_of_the_timeline() {
    let text = rendered("render-open", Some("598:"));
    assert!(text.contains("2 frames"), "got {text}");
}

/// Absent means the whole timeline, exactly as before the argument existed.
/// Asserted on a two-frame timeline of its own so it stays cheap.
#[test]
fn without_a_range_the_whole_timeline_is_rendered() {
    let dir = project("render-all");
    // Every clip cut to two frames: what is asserted is that nothing bounded
    // the render, and the fixture's own twenty seconds would be a slow way to
    // assert it.
    let document = super::fixture::DOCUMENT
        .replace("\"duration\": 600", "\"duration\": 2")
        .replace(
            "\"start\": 60, \"duration\": 90",
            "\"start\": 0, \"duration\": 2",
        );
    std::fs::write(dir.join("project.json"), document).expect("write the shortened project");
    let (text, failed) = said(&call(
        "render",
        json!({ "project": dir, "out": dir.join("cut.mp4"), "resolution": SMALL }),
    ));
    assert!(!failed, "render refused: {text}");
    assert!(text.contains("2 frames"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Refused in the CLI's words, and refused before ffmpeg is even looked for —
/// a range that is not one costs nothing to turn down.
#[test]
fn a_range_that_is_not_one_is_refused_the_way_the_command_line_refuses_it() {
    let dir = project("render-refused");
    for (range, says) in [
        (
            "2.5",
            "expected a frame range like `30:120`, `30:`, or `:120`",
        ),
        ("30:10", "frame range 30:10 ends before it begins"),
    ] {
        let (text, failed) = said(&call(
            "render",
            json!({ "project": dir, "out": dir.join("cut.mp4"), "range": range }),
        ));
        assert!(failed, "`{range}` must be refused, and got: {text}");
        assert!(
            text.contains(says),
            "`{range}` said the wrong thing: {text}"
        );
    }
    assert!(
        !dir.join("cut.mp4").exists(),
        "a refused range must leave no file behind"
    );
    std::fs::remove_dir_all(dir).ok();
}
