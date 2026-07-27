//! The containers scorsese writes, and what each one is written with.
//!
//! The lists here are what we deliver and test, not what ffmpeg tolerates.
//! ffmpeg will happily mux H.264 into ASF; the result is a `.wmv` that no
//! consumer of a Windows Media file would recognise as one, which is exactly
//! the quiet wrong answer this module exists to refuse.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use super::FormatError;
use super::codecs::{AudioCodec, VideoCodec};

/// A container a render can be delivered in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    /// MPEG-4 Part 14. What "a video file" means to almost everyone, and the
    /// one this defaults to.
    Mp4,
    /// Matroska. The open container that carries anything, for when an mp4's
    /// constraints are the problem.
    Mkv,
    /// AVI. Legacy delivery, and the reason this setting exists: asking for
    /// one should get the codecs an AVI is expected to hold.
    Avi,
    /// Windows Media, an ASF file carrying WMV/WMA. Nothing else in here is a
    /// `.wmv` in any sense a viewer of one would accept.
    Wmv,
}

impl Container {
    /// Every container, in the order they are listed to a reader.
    pub const ALL: [Self; 4] = [Self::Mp4, Self::Mkv, Self::Avi, Self::Wmv];

    /// mp4, because H.264 in an mp4 is what delivering a video means unless
    /// somebody says otherwise.
    pub const DEFAULT: Self = Self::Mp4;

    /// The name this is written and parsed as, which is also its file
    /// extension — the two agreeing is what lets an output path supply the
    /// default.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Avi => "avi",
            Self::Wmv => "wmv",
        }
    }

    /// The ffmpeg muxer for `-f`, which is not always the extension: Matroska
    /// is `matroska` and Windows Media is `asf`.
    ///
    /// Passed explicitly on every render, so the container is the setting's
    /// decision rather than an inference ffmpeg makes from the file name.
    pub const fn muxer(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "matroska",
            Self::Avi => "avi",
            Self::Wmv => "asf",
        }
    }

    /// The video codecs this container is written with, the default first.
    ///
    /// AVI listing H.264 second is the whole shape of this setting in one
    /// line: it is legal and occasionally wanted, but it is not what asking
    /// for an AVI should quietly get you.
    pub const fn video_codecs(self) -> &'static [VideoCodec] {
        match self {
            Self::Mp4 | Self::Mkv => &[VideoCodec::H264],
            Self::Avi => &[VideoCodec::Mpeg4, VideoCodec::H264],
            Self::Wmv => &[VideoCodec::Wmv2],
        }
    }

    /// The audio codecs this container is written with, the default first.
    pub const fn audio_codecs(self) -> &'static [AudioCodec] {
        match self {
            Self::Mp4 | Self::Mkv => &[AudioCodec::Aac],
            Self::Avi => &[AudioCodec::PcmS16Le],
            Self::Wmv => &[AudioCodec::Wmav2],
        }
    }

    /// What this container is written with when nobody names a codec.
    pub fn default_video(self) -> VideoCodec {
        *self
            .video_codecs()
            .first()
            .expect("every container lists at least one video codec")
    }

    /// The audio half of [`Container::default_video`].
    pub fn default_audio(self) -> AudioCodec {
        *self
            .audio_codecs()
            .first()
            .expect("every container lists at least one audio codec")
    }

    /// The container an output path's extension asks for.
    ///
    /// This is where "the file name decides" is allowed to live: as a
    /// *default* a caller can override, in one place, rather than as an
    /// inference buried in the encoder.
    pub fn from_path(path: &Path) -> Result<Self, FormatError> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| FormatError::NoExtension {
                path: path.to_path_buf(),
            })?;
        extension
            .parse()
            .map_err(|_| FormatError::UnknownExtension {
                extension: extension.to_owned(),
            })
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Container {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|container| container.name().eq_ignore_ascii_case(text.trim()))
            .ok_or_else(|| FormatError::UnknownContainer {
                name: text.trim().to_owned(),
            })
    }
}

/// Every container's name, for a refusal to say what it would have taken.
pub(super) fn names() -> String {
    Container::ALL
        .iter()
        .map(|container| container.name())
        .collect::<Vec<_>>()
        .join(", ")
}
