//! Test doubles for core's seams, for this crate's tests and every other
//! crate's, behind the `test-support` feature (#1279). Nothing that ships turns
//! it on: only a `[dev-dependencies]` entry does.

use std::path::Path;

use crate::{Fps, MediaMetadata, ProbeError, ProbeMedia};

/// A prober that answers from memory.
///
/// This is why [`ProbeMedia`] is a trait: every media-pool test runs without
/// ffmpeg installed, in microseconds, and can produce probe results — an
/// unreadable file, a video with no picture in it — that would be tedious to
/// manufacture as real media.
pub struct StubProbe {
    result: Result<MediaMetadata, String>,
}

impl StubProbe {
    /// Every file is a two-second 320×240 video at 30 fps, opaque, silent.
    pub fn video() -> Self {
        Self::reporting(MediaMetadata {
            duration_seconds: Some(2.0),
            width: Some(320),
            height: Some(240),
            frame_rate: Some(Fps::THIRTY),
            has_alpha: Some(false),
            audio_channels: None,
            sample_rate: None,
        })
    }

    /// Every file is a 64×64 still, with the frame rate and duration ffprobe
    /// invents for one.
    pub fn image() -> Self {
        Self::reporting(MediaMetadata {
            width: Some(64),
            height: Some(64),
            // ffprobe invents these for a still; import is expected to drop them.
            frame_rate: Some(Fps::PAL),
            duration_seconds: Some(0.04),
            ..MediaMetadata::default()
        })
    }

    /// Every file is three seconds of 44.1 kHz stereo sound.
    pub fn audio() -> Self {
        Self::reporting(MediaMetadata {
            duration_seconds: Some(3.0),
            audio_channels: Some(2),
            sample_rate: Some(44_100),
            ..MediaMetadata::default()
        })
    }

    /// Every file is whatever `metadata` says.
    pub fn reporting(metadata: MediaMetadata) -> Self {
        Self {
            result: Ok(metadata),
        }
    }

    /// Every probe fails, saying `message`.
    pub fn failing(message: &str) -> Self {
        Self {
            result: Err(message.to_owned()),
        }
    }
}

impl ProbeMedia for StubProbe {
    fn probe(&self, file: &Path) -> Result<MediaMetadata, ProbeError> {
        match &self.result {
            Ok(metadata) => Ok(*metadata),
            Err(message) => Err(ProbeError::new(file, message.clone())),
        }
    }
}
