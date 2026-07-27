//! Real media for a project to reference.
//!
//! Generated rather than committed: ffmpeg's synthetic sources make exact, tiny
//! files. Every invocation goes through [`Tools`], which is the workspace rule
//! for reaching ffmpeg at all — a test that shelled out directly would be the
//! first place that indirection stopped holding.

pub(crate) mod probe;

use std::path::Path;

use scorsese_render::Tools;

/// The raster the fixtures are made at: small enough that a whole render costs
/// under a second, big enough to tell a letterbox from a crop.
pub(crate) const SIZE: &str = "64x64";

pub(crate) fn tools() -> Tools {
    Tools::discover().expect("ffmpeg and ffprobe must be on PATH to run these tests")
}

/// A solid-colour silent video at `assets/{id}.mp4`, `seconds` long at 30 fps.
pub(crate) fn colour_video(tools: &Tools, root: &Path, id: &str, colour: &str, seconds: u32) {
    generate(
        tools,
        &root.join(format!("assets/{id}.mp4")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={SIZE}:d={seconds}:r=30"),
            "-pix_fmt",
            "yuv420p",
        ],
    );
}

/// The same, carrying a tone of its own — the shape that makes a video clip's
/// own sound part of the mix.
pub(crate) fn sounding_video(tools: &Tools, root: &Path, id: &str, colour: &str, seconds: u32) {
    generate(
        tools,
        &root.join(format!("assets/{id}.mp4")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={SIZE}:d={seconds}:r=30"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:duration={seconds}:sample_rate=48000"),
            "-pix_fmt",
            "yuv420p",
            "-shortest",
        ],
    );
}

/// A tone at `assets/{id}.wav`, for an audio track to carry.
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
    generate(
        tools,
        &root.join(format!("assets/{id}.png")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={SIZE}"),
            "-frames:v",
            "1",
        ],
    );
}

/// Runs ffmpeg with a synthetic input to make one fixture file.
pub(crate) fn generate(tools: &Tools, path: &Path, arguments: &[&str]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create the fixture's directory");
    }
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y"])
        .args(arguments)
        .arg(path)
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
