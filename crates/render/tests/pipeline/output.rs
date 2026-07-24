//! What comes out the far end of a whole-timeline render.

use scorsese_core::Fps;
use scorsese_render::FrameRange;

use crate::common::ffmpeg::{fixture_dir, inspect, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{BLACK, BLUE, RED, assert_colour, colour_asset, render};

#[test]
fn a_timeline_becomes_a_file_of_the_expected_length_and_rate() {
    let tools = tools();
    let dir = fixture_dir("whole");
    let red = colour_asset(&tools, &dir, "red", "64x64", 2);
    let blue = colour_asset(&tools, &dir, "blue", "64x64", 2);
    let project = project(
        vec![red, blue],
        vec![video_track(
            "v1",
            vec![clip("c1", "red", 0, 30), clip("c2", "blue", 45, 30)],
        )],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 75, "30 red + 15 black + 30 blue");
    let found = inspect(&tools, &out);
    assert_eq!(
        found.frames, 75,
        "the file holds as many frames as the report claims"
    );
    assert_eq!((found.width, found.height), (64, 64));
    assert_eq!(found.rate, "30/1");
    assert!(report.notes.is_empty(), "nothing to warn about");

    assert_colour(mean_rgb(&tools, &out, 10), RED, "the first clip");
    assert_colour(mean_rgb(&tools, &out, 35), BLACK, "the hole between them");
    assert_colour(mean_rgb(&tools, &out, 60), BLUE, "the second clip");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rendering_at_a_higher_rate_than_the_timeline_conforms() {
    let tools = tools();
    let dir = fixture_dir("conform");
    let red = colour_asset(&tools, &dir, "red", "64x64", 2);
    let project = project(
        vec![red],
        vec![video_track("v1", vec![clip("c1", "red", 0, 30)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::SIXTY);

    assert_eq!(
        report.frames, 60,
        "one timeline second is 60 frames at 60 fps"
    );
    let found = inspect(&tools, &out);
    assert_eq!(found.frames, 60);
    assert_eq!(found.rate, "60/1");
    assert_colour(mean_rgb(&tools, &out, 45), RED, "the conformed picture");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_gap_at_the_front_of_the_timeline_is_black() {
    let tools = tools();
    let dir = fixture_dir("leading-gap");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let project = project(
        vec![red],
        vec![video_track("v1", vec![clip("c1", "red", 15, 15)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 30, "the render starts at frame 0, not at 15");
    assert_colour(mean_rgb(&tools, &out, 5), BLACK, "before the first clip");
    assert_colour(mean_rgb(&tools, &out, 20), RED, "the first clip");
    std::fs::remove_dir_all(&dir).ok();
}
