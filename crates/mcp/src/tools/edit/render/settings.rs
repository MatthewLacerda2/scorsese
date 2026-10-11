//! What a render is asked to produce, read from `render`'s arguments.
//!
//! Every setting here is one of `scorsese render`'s flags, parsed by the type
//! the flag is parsed by (`Fps`, `Bitrate`, `SampleRate`, `Resolution`,
//! `FrameRange`), so a value and every refusal read the same from either
//! client. Kept apart from the tool so the settings a call builds can be held
//! to in a test without ffmpeg.

use scorsese_core::Fps;
use scorsese_core::style::Platform;
use scorsese_render::{
    Bands, Bitrate, FrameRange, LoudnessTarget, OutputFormat, RenderSettings, Resolution,
    SampleRate,
};

use super::Arguments;

/// The settings `arguments` ask for, delivered in `format`, and the stretch
/// of the timeline to render. `timeline` is the project's own rate, which a
/// render keeps unless `fps` names another.
pub(super) fn of(
    arguments: &Arguments,
    format: OutputFormat,
    timeline: Fps,
) -> Result<(RenderSettings, FrameRange), String> {
    // Every setting that only means something to a picture is refused for a
    // format with none, rather than ignored, in the words `scorsese render`
    // refuses its flags in.
    let picture = |setting| {
        format
            .picture_setting(setting)
            .map_err(|problem| format!("{problem}"))
    };
    let resolution = match (&arguments.resolution, &arguments.platform) {
        (Some(_), Some(_)) => {
            return Err("resolution and platform both size the render; give one".to_owned());
        }
        (Some(text), None) => {
            picture("a resolution")?;
            parsed(text, "resolution")?
        }
        (None, Some(text)) => {
            picture("a platform")?;
            Resolution::of(parsed::<Platform>(text, "platform")?)
        }
        (None, None) => Resolution::HD,
    };
    // The project's own grid by default: rendering at the rate the edit was
    // authored against is the one output rate needing no conform.
    let fps = match &arguments.fps {
        Some(text) => {
            picture("a frame rate")?;
            parsed(text, "fps")?
        }
        None => timeline,
    };
    let bitrate = match &arguments.bitrate {
        Some(text) => {
            picture("a video bitrate")?;
            Some(parsed::<Bitrate>(text, "bitrate")?)
        }
        None => None,
    };
    let bands = match arguments.narration_bands {
        Some(false) => {
            picture("leaving narration bands out")?;
            Bands::Omitted
        }
        Some(true) | None => Bands::Drawn,
    };
    let sample_rate = match &arguments.sample_rate {
        Some(text) => parsed::<SampleRate>(text, "sample_rate")?,
        None => SampleRate::DEFAULT,
    };
    let audio_bitrate = arguments
        .audio_bitrate
        .as_deref()
        .map(|text| parsed::<Bitrate>(text, "audio_bitrate"))
        .transpose()?;
    let loudness = arguments
        .loudness
        .map(LoudnessTarget::new)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    // `FrameRange`'s own `FromStr` is the one set of rules, so `30:`, `:120`
    // and every refusal read the same from either client.
    let range = match &arguments.range {
        Some(text) => parsed(text, "range")?,
        None => FrameRange::ALL,
    };
    let settings = RenderSettings::new(resolution, fps)
        .with_bitrate(bitrate)
        .with_audio(sample_rate, audio_bitrate)
        .with_format(format)
        .with_bands(bands)
        .with_loudness(loudness);
    Ok((settings, range))
}

/// `text` read as a `T`, a refusal naming the argument it came in.
fn parsed<T>(text: &str, argument: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|problem| format!("{argument}: {problem}"))
}
