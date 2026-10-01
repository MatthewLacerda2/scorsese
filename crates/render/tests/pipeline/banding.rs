//! A dark gradient, through a low-bitrate encode: does it come out banded?
//!
//! The one claim about gradients only a real encode can check. An 8-bit ramp
//! across a large dark area has a handful of levels to spend over hundreds of
//! rows, so undithered it is a staircase — and H.264 at a low bitrate keeps a
//! flat band flat. The compositor dithers (`scorsese_compositor::gradient` has
//! the measurements that chose how); this asserts the dither survives the
//! encoder well enough that the picture follows the ramp.
//!
//! **Measured on rows, not pixels.** Each row's luma is averaged across the
//! whole width, which averages the dither away and leaves what a viewer sees
//! from a sofa. A band edge is a row where the whole picture jumps a level at
//! once, and a band is a run of rows sitting exactly on one level. Undithered,
//! this ramp scores a 2.00-level jump and 100% flat rows; dithered, 0.28 and
//! 13% — the same on every frame from about the twentieth to the last.
//!
//! **Frame 30 of 60, not frame 0.** At 300 kbit/s x264 opens on a key frame at
//! a coarse quantiser, and that frame bands whatever is done to the source;
//! the P frames after it copy it until rate control settles, around frame 19.
//! The steady state, which is what anybody watches, is the claim.

use scorsese_core::{Asset, AssetId, Fill, Fps, Linear, Rgba, Stop};
use scorsese_render::{Bitrate, FrameRange, RenderSettings, Renderer, Resolution, Tools};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

/// Eight levels over the whole height: undithered, bands 45 rows tall.
fn backdrop() -> Asset {
    let fill = Fill::Linear(Linear {
        angle: 180.0,
        stops: vec![
            Stop::new(Rgba::opaque(0x10, 0x14, 0x18), 0.0),
            Stop::new(Rgba::opaque(0x18, 0x1c, 0x20), 1.0),
        ],
    });
    Asset::color(AssetId::new("bg"), fill)
}

/// Each row's mean luma in frame `index` of the decoded file.
fn row_means(tools: &Tools, file: &std::path::Path, index: u64) -> Vec<f64> {
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-vf", &format!("select=eq(n\\,{index})"), "-frames:v", "1"])
        .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .chunks_exact(WIDTH as usize)
        .map(|row| row.iter().map(|&y| f64::from(y)).sum::<f64>() / f64::from(WIDTH))
        .collect()
}

#[test]
fn a_dark_gradient_survives_a_low_bitrate_encode_without_banding() {
    let tools = tools();
    let dir = fixture_dir("banding");
    let project = project(
        vec![backdrop()],
        vec![video_track("v1", vec![clip("c1", "bg", 0, 60)])],
    );
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a raster");
    let bitrate = "300k".parse::<Bitrate>().expect("a bitrate");
    let settings = RenderSettings::new(resolution, Fps::THIRTY).with_bitrate(Some(bitrate));
    let out = dir.join("out.mp4");
    Renderer::new(&tools, settings)
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    let means = row_means(&tools, &out, 30);
    std::fs::remove_dir_all(&dir).ok();

    // The top and bottom few rows are where a macroblock meets the frame edge.
    let rows = &means[4..means.len() - 4];
    let jump = rows
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f64::max);
    let flat = rows
        .iter()
        .filter(|mean| (*mean - mean.round()).abs() < 0.05)
        .count() as f64
        / rows.len() as f64;
    assert!(
        jump < 1.0,
        "a band edge: the rows jump {jump:.2} levels at once"
    );
    assert!(
        flat < 0.6,
        "{:.0}% of rows sit flat on a level",
        flat * 100.0
    );
    assert!(
        rows[rows.len() - 1] > rows[0] + 4.0,
        "and it is still a ramp"
    );
}
