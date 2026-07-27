//! A prober that answers from memory.
//!
//! This is why `ProbeMedia` is a trait: every media-pool test runs without
//! ffmpeg installed, in microseconds, and can produce probe results — an
//! unreadable file, a video with no picture in it — that would be tedious to
//! manufacture as real media.
#![allow(dead_code)]

use std::path::Path;

use scorsese_core::{Fps, MediaMetadata, ProbeError, ProbeMedia};

pub(crate) struct StubProbe {
    result: Result<MediaMetadata, String>,
}

impl StubProbe {
    pub(crate) fn video() -> Self {
        Self::reporting(MediaMetadata {
            duration_seconds: Some(2.0),
            width: Some(320),
            height: Some(240),
            frame_rate: Some(Fps::THIRTY),
            audio_channels: None,
            sample_rate: None,
        })
    }

    pub(crate) fn image() -> Self {
        Self::reporting(MediaMetadata {
            width: Some(64),
            height: Some(64),
            // ffprobe invents these for a still; import is expected to drop them.
            frame_rate: Some(Fps::PAL),
            duration_seconds: Some(0.04),
            ..MediaMetadata::default()
        })
    }

    pub(crate) fn audio() -> Self {
        Self::reporting(MediaMetadata {
            duration_seconds: Some(3.0),
            audio_channels: Some(2),
            sample_rate: Some(44_100),
            ..MediaMetadata::default()
        })
    }

    pub(crate) fn reporting(metadata: MediaMetadata) -> Self {
        Self {
            result: Ok(metadata),
        }
    }

    pub(crate) fn failing(message: &str) -> Self {
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
