//! Tiny media files for tests, made by ffmpeg's synthetic sources.
//!
//! Compiled only for this crate's own tests and behind the `test-support`
//! feature, which a test crate turns on in its `[dev-dependencies]`; nothing
//! that ships ever reaches it. It lives here rather than in a test crate of its
//! own because every file it makes goes through [`Tools`], the command builder
//! every ffmpeg invocation in the workspace must use, and a fixture shelling out
//! directly would be the first place that rule stopped holding.
//!
//! Generated rather than committed: the sources make exact, tiny files, and a
//! test states the properties it relies on (length, size, level) in its call.
//! A test whose source is its own — a lavfi `sine` at ffmpeg's default eighth
//! of full scale, a test pattern — hands its arguments to [`generate`].

use std::path::Path;

use crate::Tools;

/// The binaries, or a panic saying what a test run needs on this machine.
pub fn tools() -> Tools {
    Tools::discover().expect("ffmpeg and ffprobe must be on PATH to run these tests")
}

/// Runs ffmpeg with `arguments` (a synthetic input, and whatever shapes the
/// output) to write one fixture at `path`, making its folder first.
///
/// `-nostdin` because a fixture never reads a terminal; `-y` because a test
/// that runs twice writes the same file twice.
pub fn generate(tools: &Tools, path: &Path, arguments: &[&str]) {
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

/// The sample rate a [`tone`] is written at.
pub const RATE: u32 = 48_000;

/// The lavfi graph of a [`tone`]: a sine at `hz` whose amplitude is whatever
/// `amplitude` evaluates to — a constant, or an expression of `t` that changes
/// partway through — `seconds` long at [`RATE`].
///
/// An explicit expression rather than lavfi's `sine` source, which generates
/// at an eighth of full scale: a default that is nobody's business but
/// ffmpeg's, and one a fixture should not silently inherit. Here the level is
/// a number in the test. A caller appends filters to it, or records it beside
/// a picture.
pub fn tone_graph(hz: u32, amplitude: &str, seconds: f64) -> String {
    format!("aevalsrc=exprs=({amplitude})*sin(2*PI*{hz}*t):duration={seconds}:sample_rate={RATE}")
}

/// A mono [`tone_graph`] written to `path`.
pub fn tone(tools: &Tools, path: &Path, hz: u32, amplitude: &str, seconds: f64) {
    generate(
        tools,
        path,
        &["-f", "lavfi", "-i", &tone_graph(hz, amplitude, seconds)],
    );
}

/// A solid-colour silent video at `path`: `colour` as ffmpeg names one, `size`
/// as `WxH`, `seconds` long at 30 fps in yuv420p, which every player reads.
pub fn colour_video(tools: &Tools, path: &Path, colour: &str, size: &str, seconds: u32) {
    generate(
        tools,
        path,
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={size}:d={seconds}:r=30"),
            "-pix_fmt",
            "yuv420p",
        ],
    );
}

/// A one-frame solid-colour picture at `path`, its format the extension's.
pub fn still(tools: &Tools, path: &Path, colour: &str, size: &str) {
    generate(
        tools,
        path,
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={size}"),
            "-frames:v",
            "1",
        ],
    );
}
