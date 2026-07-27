//! Every accepted combination, rendered for real and read back.
//!
//! One test per row of `docs/output-formats.md`. A list of formats nobody has
//! written a file in is a list of assumptions, which is the thing that page
//! says we will not stand behind.

use scorsese_render::{AudioCodec, Bitrate, Container, OutputFormat, SampleRate, VideoCodec};

use crate::{delivered, delivered_with};

#[track_caller]
fn assert_delivered(found: &crate::probing::Delivered, demuxer: &str, video: &str, audio: &str) {
    assert!(
        found.is(demuxer),
        "expected a {demuxer} file, ffprobe read {}",
        found.container
    );
    assert_eq!(found.video, video, "picture codec");
    assert_eq!(found.audio, audio, "sound codec");
}

#[test]
fn an_mp4_is_h264_and_aac() {
    let found = delivered("mp4", OutputFormat::default(), "out.mp4");
    assert_delivered(&found, "mp4", "h264", "aac");
}

#[test]
fn an_mkv_is_matroska_carrying_the_same_pair() {
    let found = delivered("mkv", OutputFormat::defaults_for(Container::Mkv), "out.mkv");
    assert_delivered(&found, "matroska", "h264", "aac");
}

#[test]
fn an_avi_carries_what_an_avi_is_expected_to() {
    let found = delivered("avi", OutputFormat::defaults_for(Container::Avi), "out.avi");
    assert_delivered(&found, "avi", "mpeg4", "pcm_s16le");
}

#[test]
fn a_wmv_is_an_actual_windows_media_file() {
    let found = delivered("wmv", OutputFormat::defaults_for(Container::Wmv), "out.wmv");
    assert_delivered(&found, "asf", "wmv2", "wmav2");
}

/// The one row where the codec is a decision separate from the container:
/// H.264 in an AVI is legal and occasionally wanted, so asking gets it.
#[test]
fn an_avi_can_be_asked_for_h264() {
    let format = OutputFormat::new(Container::Avi, Some(VideoCodec::H264), None)
        .expect("avi is written with h264 when asked");
    let found = delivered("avi-h264", format, "out.avi");
    assert_delivered(&found, "avi", "h264", "pcm_s16le");
}

/// The setting decides, not the file name. This is what the explicit `-f`
/// buys: without it ffmpeg would read the extension and quietly deliver an
/// mp4, and the flag would be decoration.
#[test]
fn the_container_setting_beats_the_extension() {
    let found = delivered(
        "misnamed",
        OutputFormat::defaults_for(Container::Avi),
        "out.mp4",
    );
    assert_delivered(&found, "avi", "mpeg4", "pcm_s16le");
}

/// An uncompressed track has exactly one rate, so a target bitrate against it
/// is a setting that cannot be honoured. Asking for both at once still has to
/// deliver the file rather than fail somewhere inside ffmpeg.
#[test]
fn an_audio_bitrate_against_uncompressed_sound_still_delivers() {
    let format = OutputFormat::new(Container::Avi, None, Some(AudioCodec::PcmS16Le))
        .expect("pcm is what an avi is written with");
    let bitrate: Bitrate = "192k".parse().expect("a legal bitrate");
    let settings = crate::settings(format).with_audio(SampleRate::DEFAULT, Some(bitrate));
    let found = delivered_with("avi-bitrate", settings, "out.avi");
    assert_delivered(&found, "avi", "mpeg4", "pcm_s16le");
}
