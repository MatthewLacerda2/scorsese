//! The render popup, part way through.

use scorsese_render::{Phase, Reading};

use super::drawing::window;
use super::fixture;
use super::framing::Part;

/// The popup mid-render: the file's name and folder, the bar at 45%, the
/// stage beside it and the frame count, and Stop.
///
/// Held at a fixed reading rather than drawn from a running render, which would
/// be wherever the machine had got it by the time the picture was taken. Only
/// the popup is in the picture: the bar's Render button, disabled behind it
/// while a render is under way, is the bar's business rather than this one's.
#[test]
fn the_render_popup_part_way_through() {
    let project = fixture::project("rendering");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    let reading = Reading {
        phase: Phase::Drawing,
        done: 412,
        of: 900,
    };
    harness
        .state_mut()
        .show_rendering(reading, "/home/you/Videos/whole.mp4".into());
    harness.run();
    harness.snapshot(
        Part::Dialog("Rendering"),
        "the_render_popup_part_way_through",
    );
}
