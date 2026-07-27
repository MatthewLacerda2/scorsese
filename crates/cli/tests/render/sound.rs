//! The audio settings, asserted against the stream the file carries.
//!
//! A sample rate is a property of the file being delivered rather than of the
//! edit, which is why it is a flag at all — so the thing that has to prove it
//! is the delivered file.

use std::path::PathBuf;

use crate::common::documents::{self, assets};
use crate::common::media::probe::probe;
use crate::common::media::tools;
use crate::common::{media, temp_dir};
use crate::{one_shot, render};

const SOUND: &str = "stream=sample_rate,channels,codec_name,bit_rate";

/// A shot with a tone under it on its own audio track.
fn with_sound(label: &str, video_frames: u32, audio_frames: u32) -> PathBuf {
    let dir = temp_dir(label);
    let tools = tools();
    media::colour_video(&tools, &dir, "red", "red", 4);
    media::tone(&tools, &dir, "bed", 4);
    documents::write_into(
        &dir,
        &[
            assets::raw("red", "video", "mp4"),
            assets::raw("bed", "audio", "wav"),
        ],
        &[
            documents::video_track(&[documents::lasting("c1", "red", 0, video_frames)]),
            documents::audio_track(&[documents::lasting("a1c", "bed", 0, audio_frames)]),
        ],
    );
    dir
}

#[test]
fn the_sample_rate_asked_for_is_what_the_mix_is_delivered_at() {
    // 32k rather than a plain count, so the suffix half of the parser is on
    // the path a person actually types.
    let dir = with_sound("mix-rate", 30, 30);
    let rendered = render(&dir, "cut.mp4", &["--sample-rate", "32k"]);
    // And the report says the rate it delivered at, which is the only place a
    // headless caller can learn it without probing the file itself.
    rendered.run.says("  audio 32000 Hz");
    let file = rendered.file();

    let stream = probe(&tools(), &file, "a:0", SOUND);
    assert_eq!(stream.number("sample_rate"), 32_000);
    assert_eq!(stream.get("codec_name"), "aac");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_with_nothing_audible_is_delivered_with_no_audio_stream_at_all() {
    // Not a stream of silence: an mp4 with an empty AAC track in it is a file
    // that claims to have a soundtrack.
    let dir = one_shot("no-sound", 15);
    let rendered = render(&dir, "cut.mp4", &[]);
    // The report must not announce a soundtrack the file does not have. The
    // label is not `silent`, and the needle carries a dash, because the run
    // prints the output path and a fixture's own name would match anything.
    rendered.run.says("silent —");
    rendered.run.silent_about("  audio ");
    let file = rendered.file();

    assert!(
        !probe(&tools(), &file, "a:0", SOUND).exists(),
        "a silent render must carry no audio stream"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_audio_bitrate_budget_reaches_the_encoder() {
    // Same mix twice, and the only difference is what the encoder was told it
    // could spend on it.
    let dir = with_sound("audio-bitrate", 60, 60);
    let lean = render(&dir, "lean.mp4", &["--audio-bitrate", "32k"]).file();
    let rich = render(&dir, "rich.mp4", &["--audio-bitrate", "192k"]).file();

    let tools = tools();
    let rate = |file: &std::path::Path| probe(&tools, file, "a:0", SOUND).number("bit_rate");
    assert!(
        rate(&rich) > rate(&lean) * 2,
        "192k ({}) should dwarf 32k ({})",
        rate(&rich),
        rate(&lean)
    );
    std::fs::remove_dir_all(&dir).ok();
}
