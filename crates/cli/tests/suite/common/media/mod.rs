//! Real media for a project to reference.
//!
//! Generated rather than committed, by `scorsese_render::testing`: what is
//! here only names a fixture by its asset id under `assets/` and holds the
//! raster these tests make it at.

pub(crate) mod probe;

use std::path::Path;

use scorsese_render::Tools;
use scorsese_render::testing;
#[allow(unused_imports)] // Each test binary uses its own subset.
pub(crate) use scorsese_render::testing::{generate, tools};

/// The raster the fixtures are made at: small enough that a whole render costs
/// under a second, big enough to tell a letterbox from a crop.
pub(crate) const SIZE: &str = "64x64";

/// A solid-colour silent video at `assets/{id}.mp4`, `seconds` long at 30 fps.
pub(crate) fn colour_video(tools: &Tools, root: &Path, id: &str, colour: &str, seconds: u32) {
    testing::colour_video(
        tools,
        &root.join(format!("assets/{id}.mp4")),
        colour,
        SIZE,
        seconds,
    );
}

/// The raster a fixture is made at when the point is how it compresses. A flat
/// colour at [`SIZE`] encodes to almost nothing whatever the encoder is asked
/// for, which makes it useless for telling one bitrate from another.
pub(crate) const BIG: &str = "320x240";

/// A moving test pattern at `assets/{id}.mp4`, [`BIG`] and 30 fps.
pub(crate) fn pattern_video(tools: &Tools, root: &Path, id: &str, seconds: u32) {
    generate(
        tools,
        &root.join(format!("assets/{id}.mp4")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=s={BIG}:d={seconds}:r=30"),
            "-pix_fmt",
            "yuv420p",
        ],
    );
}

/// A tone at `assets/{id}.wav`, for an audio track to carry.
///
/// lavfi's own `sine`, at its eighth of full scale, on purpose rather than
/// [`testing::tone`]'s full-scale graph: every test reading this file was
/// written against that level.
pub(crate) fn tone(tools: &Tools, root: &Path, id: &str, seconds: u32) {
    generate(
        tools,
        &root.join(format!("assets/{id}.wav")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:duration={seconds}:sample_rate=48000"),
        ],
    );
}

/// A still at `assets/{id}.png`.
pub(crate) fn still(tools: &Tools, root: &Path, id: &str, colour: &str) {
    testing::still(tools, &root.join(format!("assets/{id}.png")), colour, SIZE);
}
