//! What a render is asked to produce, read from `render`'s arguments.
//!
//! Only the reading is here: each argument is one of `scorsese render`'s
//! flags, and [`RenderAsk`] — the one home of every render's defaults and
//! refusals, shared with the CLI and the web — turns them into settings, so
//! a value and every refusal read the same from any client. Kept apart from
//! the tool so the settings a call builds can be held to in a test without
//! ffmpeg.

use scorsese_core::Fps;
use scorsese_render::{FrameRange, OutputFormat, RenderAsk, RenderSettings, Spelled};

use super::Arguments;

/// What `arguments` ask of the file, read but not yet checked against a format.
pub(super) fn ask(arguments: &Arguments) -> Result<RenderAsk, String> {
    RenderAsk::read(&Spelled {
        container: arguments.container.as_deref(),
        video_codec: arguments.video_codec.as_deref(),
        audio_codec: arguments.audio_codec.as_deref(),
        resolution: arguments.resolution.as_deref(),
        platform: arguments.platform.as_deref(),
        fps: arguments.fps.as_deref(),
        bitrate: arguments.bitrate.as_deref(),
        sample_rate: arguments.sample_rate.as_deref(),
        audio_bitrate: arguments.audio_bitrate.as_deref(),
        narration_bands: arguments.narration_bands,
        loudness: arguments.loudness,
    })
    .map_err(|problem| problem.to_string())
}

/// The settings `arguments` ask for, delivered in `format`, and the stretch
/// of the timeline to render. `timeline` is the project's own rate, which a
/// render keeps unless `fps` names another.
pub(super) fn of(
    arguments: &Arguments,
    format: OutputFormat,
    timeline: Fps,
) -> Result<(RenderSettings, FrameRange), String> {
    let settings = ask(arguments)?
        .settings(format, timeline)
        .map_err(|problem| problem.to_string())?;
    // `FrameRange`'s own `FromStr` is the one set of rules, so `30:`, `:120`
    // and every refusal read the same from either client.
    let range = match &arguments.range {
        Some(text) => text
            .parse()
            .map_err(|problem| format!("range: {problem}"))?,
        None => FrameRange::ALL,
    };
    Ok((settings, range))
}
