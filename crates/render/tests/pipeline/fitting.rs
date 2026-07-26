//! The three ways a clip can meet the raster, end to end.
//!
//! Exact pixels are the golden harness's job. What is asked here is what only a
//! real render can answer: that a source keeping its own size still comes out
//! of the pipe in step, whatever the layer beside it or the segment before it
//! was decoded at.

use std::path::Path;

use scorsese_core::{AssetKind, Fit, Fps, Project};
use scorsese_render::{FrameRange, RenderError, Renderer, Tools};

use crate::common::ffmpeg::{fixture_dir, mean_rgb, tools};
use crate::common::{clip, file_asset, fitted, project, video_track};
use crate::{RED, assert_colour, colour_asset, render, settings};

#[test]
fn filling_crops_the_overflow_rather_than_letterboxing_it() {
    // The same 32x64 source the letterbox test uses, filled instead of fitted.
    // Fitting leaves half the frame empty and averages about half red; filling
    // covers the raster and crops what hangs over, so the whole frame is red.
    let tools = tools();
    let dir = fixture_dir("fill");
    let tall = colour_asset(&tools, &dir, "red", "32x64", 1);
    let project = project(
        vec![tall],
        vec![video_track(
            "v1",
            vec![fitted(Fit::Fill, clip("c1", "red", 0, 15))],
        )],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_colour(mean_rgb(&tools, &out, 5), RED, "a filled source");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_native_clip_after_a_fitted_one_stays_in_step() {
    // The two clips decode at different sizes on the same track, so the buffer
    // reading the second is not the one that read the first. Get that wrong and
    // the frames do not fail — they slide, and the picture turns to noise.
    let tools = tools();
    let dir = fixture_dir("native-after-fit");
    let bed = colour_asset(&tools, &dir, "blue", "64x64", 1);
    let badge = colour_asset(&tools, &dir, "red", "32x32", 1);
    let project = project(
        vec![bed, badge],
        vec![video_track(
            "v1",
            vec![
                clip("c1", "blue", 0, 15),
                fitted(Fit::Native, clip("c2", "red", 15, 15)),
            ],
        )],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    // A quarter of the raster is the 32x32 badge and the rest is the black
    // canvas, so the average lands near a quarter of full red.
    let (red, _, _) = mean_rgb(&tools, &out, 20);
    assert!(
        (40..=90).contains(&red),
        "expected a quarter-frame of red, found an average of {red}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_native_source_bigger_than_the_raster_is_clipped_by_it() {
    // Native means native. A source larger than the render is not an error and
    // not silently shrunk: the canvas shows the middle of it.
    let tools = tools();
    let dir = fixture_dir("native-big");
    let huge = colour_asset(&tools, &dir, "red", "256x256", 1);
    let project = project(
        vec![huge],
        vec![video_track(
            "v1",
            vec![fitted(Fit::Native, clip("c1", "red", 0, 15))],
        )],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_colour(
        mean_rgb(&tools, &out, 5),
        RED,
        "a source larger than the raster",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_native_clip_whose_size_cannot_be_established_is_refused() {
    // The one thing native cannot do without: a size. Refusing before anything
    // spawns beats decoding a pipe at a stride nobody could work out — which
    // would produce a file, and a wrong one.
    let tools = tools();
    let dir = fixture_dir("native-unknown");
    std::fs::create_dir_all(dir.join("assets")).expect("create assets dir");
    std::fs::write(dir.join("assets").join("broken.mp4"), b"not a video at all")
        .expect("write the broken file");
    let project = project(
        vec![file_asset("broken", AssetKind::Video)],
        vec![video_track(
            "v1",
            vec![fitted(Fit::Native, clip("c1", "broken", 0, 15))],
        )],
    );

    let error = render_expecting_failure(&tools, &project, &dir);

    match error {
        RenderError::UnknownSourceSize { clip, asset, .. } => {
            assert_eq!((clip.as_str(), asset.as_str()), ("c1", "broken"));
        }
        other => panic!("got {other}"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

fn render_expecting_failure(tools: &Tools, project: &Project, dir: &Path) -> RenderError {
    Renderer::new(tools, settings(Fps::THIRTY))
        .render(project, dir, FrameRange::ALL, &dir.join("out.mp4"))
        .expect_err("the render cannot succeed")
}
