//! What a finished render says about itself.
//!
//! `scorsese render` and the MCP `render` tool are two ways of asking for the
//! same file, so they say the same things about it — in these words, from
//! here. They once did not: the MCP reply named the path and the format and
//! nothing about sound, so an agent could not tell its soundtrack had been
//! turned down 4 dB to fit a lossy codec without a second call it had no
//! reason to make (#519).

use crate::report::RenderReport;

/// What was written, and how much of it: `90 frames at 30 fps, 1920x1080
/// (3.00s)`, or `3.00s of sound, no picture` for a sound-only file.
pub fn written(report: &RenderReport) -> String {
    match report.resolution {
        Some(resolution) => format!(
            "{} frames at {} fps, {resolution} ({:.2}s)",
            report.frames,
            report.fps,
            report.seconds()
        ),
        None => format!("{:.2}s of sound, no picture", report.seconds()),
    }
}

/// How the soundtrack came out of the delivered file, then what was done to
/// keep it under full scale when anything was — one line each, and none at
/// all for a silent render.
///
/// The file's level rather than the mix's, because the two differ whenever
/// the codec is lossy and the file's is the one a listener hears.
pub fn delivery(report: &RenderReport) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(delivered) = &report.delivered {
        lines.push(format!("file   {}", super::loudness(delivered)));
    }
    if let Some(trim) = &report.trim {
        lines.push(format!("note: the soundtrack was {trim}"));
    }
    lines
}
