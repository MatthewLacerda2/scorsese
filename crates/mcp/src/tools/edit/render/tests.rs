//! The settings a `render` call builds (#1069): each of `scorsese render`'s
//! delivery flags reaches the `RenderSettings` as the CLI would build it, and
//! is refused in the same words. No ffmpeg: nothing here encodes.

use std::path::Path;

use scorsese_core::Fps;
use scorsese_render::{Bitrate, FrameRange, OutputFormat, RenderSettings, Resolution, SampleRate};
use serde_json::{Value, json};

use super::{Arguments, settings};
use crate::tools::args;

/// What `extra` asks of a render of a 30 fps project into `out`.
fn asked(out: &str, extra: Value) -> Result<(RenderSettings, FrameRange), String> {
    let mut arguments = json!({ "project": "settings.scor", "out": out });
    for (key, value) in extra.as_object().expect("extra arguments are an object") {
        arguments[key] = value.clone();
    }
    let arguments: Arguments = args::parse(&arguments)?;
    let format = OutputFormat::for_path(Path::new(out), None, None, None).expect("a format");
    settings::of(
        &arguments,
        format,
        &scorsese_core::Project::new("t", Fps::THIRTY),
    )
}

fn built(extra: Value) -> RenderSettings {
    asked("cut.mp4", extra).expect("accepted").0
}

#[test]
fn without_settings_a_render_keeps_the_project_rate_and_the_defaults() {
    let format = OutputFormat::for_path(Path::new("cut.mp4"), None, None, None).expect("mp4");
    let expected = RenderSettings::new(Resolution::HD, Fps::THIRTY).with_format(format);
    assert_eq!(asked("cut.mp4", json!({})), Ok((expected, FrameRange::ALL)));
}

#[test]
fn fps_bitrate_and_both_audio_settings_reach_the_render() {
    let settings = built(json!({
        "fps": "30000/1001", "bitrate": "8M", "sample_rate": "44100", "audio_bitrate": "192k"
    }));
    assert_eq!(settings.fps, Fps::NTSC);
    assert_eq!(settings.bitrate, Some("8M".parse::<Bitrate>().expect("8M")));
    assert_eq!(
        settings.sample_rate,
        SampleRate::new(44_100).expect("44.1 kHz")
    );
    assert_eq!(settings.audio_bitrate, "192k".parse().ok());
    assert_eq!(built(json!({ "fps": "60" })).fps, Fps::SIXTY);
}

/// Parsed by the CLI's own types, so the refusal is theirs, named for the
/// argument it came in.
#[test]
fn a_setting_that_does_not_parse_is_refused_in_the_types_words() {
    let refused = |extra| asked("cut.mp4", extra).expect_err("refused");
    let decimal = "29.97"
        .parse::<Fps>()
        .expect_err("a decimal rate")
        .to_string();
    assert_eq!(
        refused(json!({ "fps": "29.97" })),
        format!("fps: {decimal}")
    );
    assert!(refused(json!({ "bitrate": "0" })).starts_with("bitrate: "));
    assert!(refused(json!({ "audio_bitrate": "loud" })).starts_with("audio_bitrate: "));
    assert!(refused(json!({ "sample_rate": "0" })).starts_with("sample_rate: "));
}

/// A sound-only file has no picture for a rate or a bitrate to mean anything
/// to, so both are refused the way resolution is — and the sound settings are
/// not.
#[test]
fn a_sound_only_file_refuses_fps_and_bitrate_but_takes_the_sound_settings() {
    for (argument, setting) in [("fps", "60"), ("bitrate", "8M"), ("resolution", "640x360")] {
        let refusal = asked("score.mp3", json!({ argument: setting })).expect_err("refused");
        assert!(refusal.contains("mp3"), "{argument}: {refusal}");
    }
    let (settings, _) = asked(
        "score.mp3",
        json!({ "sample_rate": "44100", "audio_bitrate": "320k" }),
    )
    .expect("sound settings are taken");
    assert_eq!(settings.sample_rate.hz(), 44_100);
    assert_eq!(settings.audio_bitrate, "320k".parse().ok());
}
