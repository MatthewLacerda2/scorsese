//! Image formats that carry an animation, and what a render does with one.
//!
//! A gif is an `image` asset like a png is, so it is *held* for the clip's
//! length rather than played against a timeline of its own. Held is not frozen:
//! the picture it holds moves, and when the clip outlasts the animation it
//! starts again. Both halves are asserted here because both were broken — a gif
//! did not animate, it failed the render outright, on a decoder option its
//! demuxer does not accept.
//!
//! Needs ffmpeg on PATH, like everything else here that touches real media.

#[path = "common/mod.rs"]
mod common;

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};
use scorsese_render::{FrameRange, RenderSettings, Renderer, Resolution, Tools};

use common::ffmpeg::{fixture_dir, generate, mean_rgb, tools};
use common::{clip, project, video_track};

const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const GREEN: (u8, u8, u8) = (0, 128, 0);

fn settings() -> RenderSettings {
    RenderSettings::new(
        Resolution::new(32, 32).expect("32x32 is a resolution"),
        Fps::THIRTY,
    )
}

/// A gif of `seconds` seconds per colour, at 10fps, in the project's assets.
fn gif(tools: &Tools, root: &Path, name: &str, colours: &[&str]) -> Asset {
    let inputs: Vec<String> = colours
        .iter()
        .map(|colour| format!("color=c={colour}:s=32x32:d=0.5:r=10"))
        .collect();
    let mut args: Vec<&str> = Vec::new();
    for input in &inputs {
        args.extend(["-f", "lavfi", "-i", input]);
    }
    let labels: String = (0..colours.len()).map(|at| format!("[{at}:v]")).collect();
    let concat = format!("{labels}concat=n={}:v=1", colours.len());
    if colours.len() > 1 {
        args.extend(["-filter_complex", &concat]);
    }
    generate(
        tools,
        &root.join("assets").join(format!("{name}.gif")),
        &args,
    );
    Asset::imported(
        AssetId::new(name),
        AssetKind::Image,
        ProjectPath::new(format!("assets/{name}.gif")),
    )
}

#[track_caller]
fn assert_colour(found: (u8, u8, u8), expected: (u8, u8, u8), what: &str) {
    let close = |a: u8, b: u8| a.abs_diff(b) <= 24;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1) && close(found.2, expected.2),
        "{what}: expected {expected:?}, found {found:?}"
    );
}

/// Renders the project and reports the colour of one frame of the file it
/// wrote. The delivered file is the only place the question can be settled: a
/// preview composites the one frame it was asked for, and a held source starts
/// at its beginning for every segment that decodes it.
fn rendered_frame(project: &Project, root: &Path, tools: &Tools, at: u64) -> (u8, u8, u8) {
    let out = root.join("out.mp4");
    if !out.exists() {
        Renderer::new(tools, settings())
            .render(project, root, FrameRange::ALL, &out)
            .expect("the render succeeds");
    }
    mean_rgb(tools, &out, at)
}

/// Red for the first half-second, blue for the second: a gif whose picture
/// changes, held under a clip that outlasts it.
#[test]
fn an_animated_image_plays_and_then_repeats() {
    let tools = tools();
    let dir = fixture_dir("animated-gif");
    let asset = gif(&tools, &dir, "wave", &["red", "blue"]);
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", "wave", 0, 45)])],
    );

    assert_colour(
        rendered_frame(&project, &dir, &tools, 0),
        RED,
        "the opening frame",
    );
    assert_colour(
        rendered_frame(&project, &dir, &tools, 20),
        BLUE,
        "two thirds of a second in, the animation has moved on",
    );
    assert_colour(
        rendered_frame(&project, &dir, &tools, 40),
        RED,
        "past the gif's own length, so it is playing again from the top",
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A gif of one frame is a still like any other. This failed the render before
/// the hold stopped depending on a demuxer option gif does not have.
#[test]
fn a_single_frame_gif_is_held_like_any_other_still() {
    let tools = tools();
    let dir = fixture_dir("static-gif");
    let asset = gif(&tools, &dir, "card", &["green"]);
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", "card", 0, 30)])],
    );

    assert_colour(
        rendered_frame(&project, &dir, &tools, 0),
        GREEN,
        "the first frame",
    );
    assert_colour(
        rendered_frame(&project, &dir, &tools, 25),
        GREEN,
        "a later frame",
    );
    std::fs::remove_dir_all(&dir).ok();
}
