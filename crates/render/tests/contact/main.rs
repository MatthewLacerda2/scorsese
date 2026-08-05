//! Looking at a video file: which moments a sheet takes, and what comes back.
//!
//! Needs ffmpeg on PATH, like everything else here that touches real media.
//!
//! The fixture is built so a wrong moment is a wrong *colour*: the file changes
//! colour every five seconds, so a sheet that sampled the wrong instants comes
//! back looking perfectly fine and is wrong in a way only a pixel can catch.

#[path = "../common/mod.rs"]
mod common;

mod sampling;
mod sheets;

use std::path::{Path, PathBuf};

use scorsese_render::Tools;

use common::ffmpeg::generate;

/// Five seconds each of red, green and blue: a 15-second file whose colour
/// says which five-second band a frame came from.
fn banded(tools: &Tools, dir: &Path) -> PathBuf {
    let file = dir.join("banded.mp4");
    generate(
        tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x32:d=5:r=10",
            "-f",
            "lavfi",
            "-i",
            "color=c=green:s=64x32:d=5:r=10",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=64x32:d=5:r=10",
            "-filter_complex",
            "[0][1][2]concat=n=3:v=1:a=0",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    file
}
