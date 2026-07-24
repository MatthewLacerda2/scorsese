//! Probing real media with real ffprobe.
//!
//! These tests need ffmpeg on PATH — it is a stated dev/CI requirement, and
//! CI installs it. They deliberately do not skip themselves when it is
//! absent: a test that quietly passes by doing nothing is worse than one that
//! fails loudly.
//!
//! The fixtures are generated here rather than committed. ffmpeg's own
//! synthetic sources make exact, tiny media, and generating them through
//! [`Tools`] exercises the command builder that every ffmpeg invocation in
//! the workspace must go through.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::Fps;
use scorsese_core::ProbeMedia;
use scorsese_render::{Ffprobe, Tools};

fn tools() -> Tools {
    Tools::discover().expect("ffmpeg and ffprobe must be on PATH to run these tests")
}

fn fixture_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-probe-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    dir
}

/// Runs ffmpeg with a synthetic input to make one small fixture file.
fn generate(tools: &Tools, path: &Path, args: &[&str]) {
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(path)
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_video_reports_size_rate_and_duration() {
    let tools = tools();
    let dir = fixture_dir("video");
    let file = dir.join("clip.mp4");
    generate(
        &tools,
        &file,
        &["-f", "lavfi", "-i", "color=c=teal:s=320x240:d=2:r=30"],
    );

    let media = Ffprobe::new(tools).probe(&file).expect("probe");

    assert_eq!((media.width, media.height), (Some(320), Some(240)));
    assert_eq!(media.frame_rate, Some(Fps::THIRTY));
    let duration = media.duration_seconds.expect("a duration");
    assert!((duration - 2.0).abs() < 0.1, "duration was {duration}");
    assert_eq!(media.audio_channels, None, "this clip is silent");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn audio_reports_channels_and_sample_rate() {
    let tools = tools();
    let dir = fixture_dir("audio");
    let file = dir.join("beep.wav");
    generate(
        &tools,
        &file,
        &["-f", "lavfi", "-i", "sine=frequency=440:duration=3"],
    );

    let media = Ffprobe::new(tools).probe(&file).expect("probe");

    assert_eq!(media.audio_channels, Some(1));
    assert_eq!(media.sample_rate, Some(44_100));
    assert_eq!(media.width, None, "there is no picture in a wav");
    let duration = media.duration_seconds.expect("a duration");
    assert!((duration - 3.0).abs() < 0.1, "duration was {duration}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_image_has_size_but_no_real_frame_rate() {
    // ffprobe describes a PNG as a one-frame video. What matters is that the
    // dimensions survive; import is what discards the invented timing.
    let tools = tools();
    let dir = fixture_dir("image");
    let file = dir.join("logo.png");
    generate(
        &tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x64:d=1",
            "-frames:v",
            "1",
        ],
    );

    let media = Ffprobe::new(tools).probe(&file).expect("probe");

    assert_eq!((media.width, media.height), (Some(64), Some(64)));
    assert_eq!(media.audio_channels, None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn probing_something_that_is_not_media_fails_with_a_reason() {
    let dir = fixture_dir("garbage");
    let file = dir.join("not-media.mp4");
    std::fs::write(&file, b"this is not a video").expect("write");

    let error = Ffprobe::new(tools()).probe(&file).expect_err("must fail");

    assert_eq!(error.path, file);
    assert!(
        !error.message.is_empty(),
        "the failure says something useful"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn probing_a_missing_file_fails() {
    let error = Ffprobe::new(tools())
        .probe(Path::new("/definitely/not/here.mp4"))
        .expect_err("must fail");
    assert!(!error.message.is_empty());
}
