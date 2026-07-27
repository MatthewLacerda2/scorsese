//! The combinations we will not deliver, refused where nothing has been spent.
//!
//! Every test here is a pure call: [`OutputFormat::new`] takes no tools, no
//! project and no output path, so there is nothing it could have spawned or
//! written. That is what makes the refusal early — not the order of statements
//! in some caller, but the fact that the type cannot exist for a combination
//! we do not write, and the encoder only ever sees the type.
//!
//! The end-to-end half of that claim — that `scorsese render` refuses before
//! ffmpeg is so much as located — is `crates/cli/tests/render.rs`.

use std::path::Path;

use scorsese_render::{AudioCodec, Container, FormatError, OutputFormat, VideoCodec};

/// The file the issue was written about: an ASF carrying H.264, handed back
/// after a full encode as though it were a Windows Media file.
#[test]
fn a_wmv_is_not_written_with_h264() {
    let refused = OutputFormat::new(Container::Wmv, Some(VideoCodec::H264), None);
    assert_eq!(
        refused,
        Err(FormatError::VideoNotWritten {
            container: Container::Wmv,
            video: VideoCodec::H264,
        })
    );
    let said = refused.expect_err("refused").to_string();
    assert!(
        said.contains("wmv2"),
        "the refusal says what it would take: {said}"
    );
}

#[test]
fn an_mp4_is_not_written_with_windows_media_codecs() {
    assert!(OutputFormat::new(Container::Mp4, Some(VideoCodec::Wmv2), None).is_err());
    assert!(OutputFormat::new(Container::Mp4, None, Some(AudioCodec::Wmav2)).is_err());
}

/// The audio half is checked as thoroughly as the picture half: a container
/// that carried the right picture and the wrong sound would be exactly as
/// unplayable.
#[test]
fn an_mp4_is_not_written_with_uncompressed_sound() {
    let refused = OutputFormat::new(Container::Mp4, None, Some(AudioCodec::PcmS16Le));
    assert_eq!(
        refused,
        Err(FormatError::AudioNotWritten {
            container: Container::Mp4,
            audio: AudioCodec::PcmS16Le,
        })
    );
}

#[test]
fn an_avi_is_not_written_with_windows_media_sound() {
    assert!(OutputFormat::new(Container::Avi, None, Some(AudioCodec::Wmav2)).is_err());
}

/// Refused rather than handed to ffmpeg to interpret: guessing from the file
/// name is the behaviour being removed, not one to fall back on.
#[test]
fn an_extension_we_do_not_write_is_refused() {
    let refused = Container::from_path(Path::new("cut.mov"));
    assert_eq!(
        refused,
        Err(FormatError::UnknownExtension {
            extension: "mov".to_owned(),
        })
    );
    let said = refused.expect_err("refused").to_string();
    assert!(
        said.contains("mp4"),
        "the refusal lists what we do write: {said}"
    );
}

#[test]
fn a_path_with_no_extension_has_nothing_to_infer_from() {
    assert!(matches!(
        Container::from_path(Path::new("cut")),
        Err(FormatError::NoExtension { .. })
    ));
}

#[test]
fn a_name_that_is_not_a_container_or_a_codec_is_refused() {
    assert!("webm".parse::<Container>().is_err());
    assert!("vp9".parse::<VideoCodec>().is_err());
    assert!("opus".parse::<AudioCodec>().is_err());
}
