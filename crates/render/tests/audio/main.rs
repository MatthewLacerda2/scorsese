//! Rendering projects that have sound in them, and listening to the result.
//!
//! These need ffmpeg on PATH — a stated dev/CI requirement, which CI installs.
//! They deliberately do not skip themselves when it is absent: a test that
//! quietly passes by doing nothing is worse than one that fails loudly.
//!
//! There are no committed reference waveforms. A soundtrack that has been
//! resampled and encoded is not reproducible sample for sample, and a reference
//! nobody can read is a reference nobody can check — so every assertion here is
//! a question with an obvious answer instead: is there sound at this second,
//! and is it getting louder?

#[path = "../common/mod.rs"]
mod common;

mod mixing;
mod sources;
mod volume;

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetKind, Fps, Project};
use scorsese_render::{FrameRange, RenderReport, RenderSettings, Renderer, Resolution, Tools};

use common::ffmpeg::generate_asset;

/// Small, quick, and irrelevant: nothing here looks at the picture, but a
/// render needs one because picture is what decides a render's length.
const RASTER: (u32, u32) = (32, 32);

/// A silent video asset to hang a soundtrack on.
pub fn picture(tools: &Tools, root: &Path, seconds: u32) -> Asset {
    generate_asset(
        tools,
        root,
        "picture",
        AssetKind::Video,
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:s=32x32:d={seconds}:r=30"),
            "-pix_fmt",
            "yuv420p",
        ],
    )
}

/// Renders a project into its own directory and hands back the file it wrote.
pub fn render(tools: &Tools, project: &Project, root: &Path) -> (PathBuf, RenderReport) {
    let out = root.join("out.mp4");
    let resolution =
        Resolution::new(RASTER.0, RASTER.1).expect("the fixture raster is a legal one");
    let report = Renderer::new(tools, RenderSettings::new(resolution, Fps::THIRTY))
        .render(project, root, FrameRange::ALL, &out)
        .expect("the render succeeds");
    (out, report)
}
