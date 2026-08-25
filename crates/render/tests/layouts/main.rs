//! Where each clip's content lands, asked of the document alone.
//!
//! No ffmpeg anywhere here, which is half the point of the feature: the
//! rectangle a title's block covers is knowable before anything is composited,
//! and it is what authoring a layout used to cost a render and a pair of eyes.

#[path = "../common/mod.rs"]
mod common;

mod absent;
mod pivoted;
mod placed;
mod reading;

use std::path::Path;

use scorsese_core::{Frames, Project};
use scorsese_render::{Layout, Region, Resolution};

/// The raster every question here is asked against — 16:9, so a letterbox has
/// somewhere to go and a square source is visibly not the frame.
fn raster() -> Resolution {
    "1920x1080".parse().expect("a literal raster parses")
}

/// The layout of a fixture project at one timeline frame.
///
/// The project root is the working directory, which nothing in these fixtures
/// reads: every asset here carries its content in the document.
fn layout(project: &Project, at: u64) -> Layout {
    Layout::at(project, Path::new("."), raster(), Frames(at))
        .expect("the fixture timeline sequences")
}

/// One clip's rectangle at one instant.
fn region(project: &Project, at: u64, clip: &str) -> Region {
    layout(project, at)
        .of(clip)
        .unwrap_or_else(|| panic!("`{clip}` is on screen at frame {at}"))
}

/// A rectangle rounded to the thousandth, which is the precision the answer is
/// printed to and finer than a pixel at 1080p.
fn rounded(region: Region) -> (f64, f64, f64, f64) {
    let round = |value: f64| (value * 1000.0).round() / 1000.0;
    (
        round(region.left),
        round(region.top),
        round(region.width),
        round(region.height),
    )
}
