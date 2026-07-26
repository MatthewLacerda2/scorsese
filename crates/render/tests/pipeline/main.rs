//! Rendering real projects to real files with real ffmpeg.
//!
//! These need ffmpeg on PATH — a stated dev/CI requirement, which CI installs.
//! They deliberately do not skip themselves when it is absent: a test that
//! quietly passes by doing nothing is worse than one that fails loudly.
//!
//! What they assert is the shape of the output — frame count, rate, raster —
//! and the average colour of individual frames, which is enough to tell red
//! from black. Committed reference frames and SSIM tolerances are the
//! golden-render harness's job, not this one's.

#[path = "../common/mod.rs"]
mod common;

mod fitting;
mod output;
mod partial;
mod sources;
mod warnings;

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetKind, Fps, Project};
use scorsese_render::{FrameRange, RenderReport, RenderSettings, Renderer, Resolution, Tools};

use common::ffmpeg::generate_asset;

/// The raster the fixtures render at: small enough to be quick, big enough for
/// letterboxing to be visible.
const RASTER: (u32, u32) = (64, 64);

pub const BLACK: (u8, u8, u8) = (0, 0, 0);
pub const RED: (u8, u8, u8) = (254, 0, 0);
pub const GREEN: (u8, u8, u8) = (0, 128, 0);
pub const BLUE: (u8, u8, u8) = (0, 0, 254);

fn settings(fps: Fps) -> RenderSettings {
    let resolution =
        Resolution::new(RASTER.0, RASTER.1).expect("the fixture raster is a legal one");
    RenderSettings::new(resolution, fps)
}

/// An ffmpeg lavfi description of a solid colour at 30 fps.
fn colour_filter(colour: &str, size: &str, seconds: u32) -> String {
    format!("color=c={colour}:s={size}:d={seconds}:r=30")
}

/// A solid-colour video asset in the project's `assets/`, named after its own
/// colour so a test's expectations read off its fixtures.
fn colour_asset(tools: &Tools, root: &Path, colour: &str, size: &str, seconds: u32) -> Asset {
    generate_asset(
        tools,
        root,
        colour,
        AssetKind::Video,
        &[
            "-f",
            "lavfi",
            "-i",
            &colour_filter(colour, size, seconds),
            "-pix_fmt",
            "yuv420p",
        ],
    )
}

/// Renders a project into its own directory and hands back the file it wrote.
fn render(
    tools: &Tools,
    project: &Project,
    root: &Path,
    range: FrameRange,
    fps: Fps,
) -> (PathBuf, RenderReport) {
    let out = root.join("out.mp4");
    let report = Renderer::new(tools, settings(fps))
        .render(project, root, range, &out)
        .expect("the render succeeds");
    (out, report)
}

/// How close two colours have to be to count as the same one.
///
/// Generous on purpose: frames make a round trip through 4:2:0 chroma
/// subsampling and a lossy encoder, so exact equality would be asserting
/// something about libx264 rather than about scorsese.
#[track_caller]
fn assert_colour(found: (u8, u8, u8), expected: (u8, u8, u8), what: &str) {
    let close = |a: u8, b: u8| a.abs_diff(b) <= 12;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1) && close(found.2, expected.2),
        "{what}: expected about {expected:?}, found {found:?}"
    );
}
