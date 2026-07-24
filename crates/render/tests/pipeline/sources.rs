//! How sources that are not simply "video of the right shape" are handled.

use scorsese_core::{AssetKind, Fps};
use scorsese_render::FrameRange;

use crate::common::ffmpeg::{fixture_dir, generate_asset, inspect, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{BLUE, assert_colour, colour_asset, colour_filter, render};

#[test]
fn a_source_of_another_shape_is_letterboxed_not_stretched() {
    // A 32x64 source in a 64x64 raster keeps its proportions and gains black at
    // the sides, so half the width is the source's colour and half is black —
    // an average of about 127. Stretching it would make the whole frame red.
    let tools = tools();
    let dir = fixture_dir("letterbox");
    let tall = colour_asset(&tools, &dir, "red", "32x64", 1);
    let project = project(
        vec![tall],
        vec![video_track("v1", vec![clip("c1", "red", 0, 15)])],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    let (red, _, _) = mean_rgb(&tools, &out, 5);
    assert!(
        (100..=155).contains(&red),
        "expected about half a frame of red, found an average of {red}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_image_is_held_for_the_clips_length() {
    // A still has no timeline of its own, so how long it is on screen is the
    // clip's business: one frame of source fills as many frames as asked for.
    let tools = tools();
    let dir = fixture_dir("still");
    let logo = generate_asset(
        &tools,
        &dir,
        "logo",
        AssetKind::Image,
        &[
            "-f",
            "lavfi",
            "-i",
            &colour_filter("blue", "64x64", 1),
            "-frames:v",
            "1",
        ],
    );
    let project = project(
        vec![logo],
        vec![video_track("v1", vec![clip("c1", "logo", 0, 45)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 45, "a one-frame source fills 45 frames");
    assert_eq!(inspect(&tools, &out).frames, 45);
    assert_colour(mean_rgb(&tools, &out, 44), BLUE, "the last frame");
    assert!(
        report.notes.is_empty(),
        "a still is never short of source, however long the clip"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_source_shot_at_another_rate_is_conformed_onto_the_timeline_grid() {
    // 24 fps source, 30 fps timeline: the clip's length is decided by the
    // timeline, and the source's own rate has nothing to say about it.
    let tools = tools();
    let dir = fixture_dir("source-rate");
    let slow = generate_asset(
        &tools,
        &dir,
        "green",
        AssetKind::Video,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=green:s=64x64:d=2:r=24",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let project = project(
        vec![slow],
        vec![video_track("v1", vec![clip("c1", "green", 0, 45)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 45, "45 timeline frames is 45 output frames");
    assert_eq!(inspect(&tools, &out).frames, 45);
    assert!(
        report.notes.is_empty(),
        "two seconds of 24 fps source covers 45 frames of a 30 fps timeline"
    );
    std::fs::remove_dir_all(&dir).ok();
}
