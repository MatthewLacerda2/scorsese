//! A platform's interface laid over the finished frames (#1010): over the
//! picture in a still and in the file, and nowhere in the edit itself.

use scorsese_core::style::Platform;
use scorsese_core::{Clip, Fps, Frames, Project};
use scorsese_render::{Frame, FrameRange, Note, Overlay, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{mean_rgb, tools};
use crate::common::scratch;
use crate::common::{project, video_track};
use crate::pipeline::{RED, assert_colour, colour_asset};

const UPRIGHT: (u32, u32) = (108, 192);

fn rgb(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * UPRIGHT.0 + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

fn settings(overlay: Option<Overlay>) -> RenderSettings {
    let resolution = Resolution::new(UPRIGHT.0, UPRIGHT.1).expect("an upright raster");
    RenderSettings::new(resolution, Fps::THIRTY).with_overlay(overlay)
}

fn red(dir: &std::path::Path) -> Project {
    let asset = colour_asset(&tools(), dir, "red", "108x192", 1);
    project(
        vec![asset],
        vec![video_track("v1", vec![Clip::at("shot", "red", 0, 10)])],
    )
}

#[test]
fn the_interface_is_laid_over_the_still_and_the_file_and_the_shot_shows_between() {
    let tools = tools();
    let dir = scratch::dir("overlay");
    let project = red(&dir);
    let reels = Overlay::new(Platform::InstagramReels, false).expect("a feed");

    let plain = Renderer::new(&tools, settings(None));
    let overlaid = Renderer::new(&tools, settings(Some(reels)));
    let still = overlaid.still(&project, &dir, Frames(5)).expect("a still");
    let (foot, middle) = (UPRIGHT.1 - 2, UPRIGHT.1 * 2 / 5);
    assert!(rgb(&still, 2, foot).0 < 40, "the navigation bar is dark");
    assert_colour(
        rgb(&still, 20, middle),
        RED,
        "the shot between the controls",
    );
    let bare = plain.still(&project, &dir, Frames(5)).expect("a still");
    assert_colour(rgb(&bare, 2, foot), RED, "nothing over a plain still");

    let (with, without) = (dir.join("with.mp4"), dir.join("without.mp4"));
    let report = overlaid
        .render(&project, &dir, FrameRange::ALL, &with)
        .expect("the render succeeds");
    assert!(
        report.notes.contains(&Note::Overlaid(reels)),
        "the report says the file is a preview: {:?}",
        report.notes
    );
    plain
        .render(&project, &dir, FrameRange::ALL, &without)
        .expect("the render succeeds");
    let (shaded, _, _) = mean_rgb(&tools, &with, 3);
    let (clean, _, _) = mean_rgb(&tools, &without, 3);
    assert!(
        shaded + 30 < clean,
        "the file carries it: {shaded} against {clean}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
