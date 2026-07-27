//! The encoders scorsese will ask ffmpeg for.
//!
//! Every one of these is either already required by the default path
//! (`libx264`) or built into ffmpeg itself with no external library behind it
//! (`mpeg4`, `wmv2`, `aac`, `pcm_s16le`, `wmav2`). That is deliberate: which
//! ffmpeg we are talking to is a shipping decision — a distro build in dev and
//! CI, a bundled sidecar in a shipped build — so a codec list that needs a
//! particular build is a list we cannot stand behind.

use std::fmt;
use std::str::FromStr;

use super::FormatError;

/// A video encoder a render can be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    /// H.264/AVC, via `libx264`. What delivery means nearly always, and what
    /// every player made in the last fifteen years decodes.
    H264,
    /// MPEG-4 Part 2, via ffmpeg's own `mpeg4`. What an `.avi` is expected to
    /// carry — the DivX/Xvid lineage that made the container common.
    Mpeg4,
    /// Windows Media Video 8, via ffmpeg's own `wmv2`. The only thing that
    /// makes a `.wmv` a Windows Media file rather than an ASF wrapper around
    /// something else.
    Wmv2,
}

impl VideoCodec {
    /// Every codec, in the order they are listed to a reader.
    pub const ALL: [Self; 3] = [Self::H264, Self::Mpeg4, Self::Wmv2];

    /// The name this is written and parsed as, on the command line and in
    /// `docs/output-formats.md`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::H264 => "h264",
            Self::Mpeg4 => "mpeg4",
            Self::Wmv2 => "wmv2",
        }
    }

    /// The ffmpeg encoder, which is not always the codec's own name —
    /// H.264 is encoded by `libx264`.
    pub const fn encoder(self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::Mpeg4 => "mpeg4",
            Self::Wmv2 => "wmv2",
        }
    }

    /// How to ask this encoder for constant quality, for the renders with no
    /// file-size budget. The knob is per-encoder: `-crf` is x264's and means
    /// nothing to the other two, which take a quantiser scale instead.
    ///
    /// Both values are near the top of their scale's useful range — visually
    /// transparent for the flat graphics and titles this composites, without
    /// spending bits nobody asked for.
    pub const fn quality(self) -> [&'static str; 2] {
        match self {
            Self::H264 => ["-crf", "18"],
            Self::Mpeg4 | Self::Wmv2 => ["-q:v", "4"],
        }
    }
}

impl fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for VideoCodec {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|codec| codec.name().eq_ignore_ascii_case(text.trim()))
            .ok_or_else(|| FormatError::UnknownVideoCodec {
                name: text.trim().to_owned(),
            })
    }
}

/// An audio encoder a render can be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioCodec {
    /// AAC, via ffmpeg's own encoder. Transparent for speech and music beds at
    /// the rates anyone would pick, and decoded by everything.
    Aac,
    /// Uncompressed 16-bit PCM. What an `.avi` is expected to carry, and the
    /// answer when a file is going into another tool rather than to a viewer.
    PcmS16Le,
    /// Windows Media Audio 2, the sound half of a real `.wmv`.
    Wmav2,
}

impl AudioCodec {
    /// Every codec, in the order they are listed to a reader.
    pub const ALL: [Self; 3] = [Self::Aac, Self::PcmS16Le, Self::Wmav2];

    /// The name this is written and parsed as, which is also ffmpeg's encoder
    /// name — none of these come from an external library.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::PcmS16Le => "pcm_s16le",
            Self::Wmav2 => "wmav2",
        }
    }

    /// The ffmpeg encoder to ask for.
    pub const fn encoder(self) -> &'static str {
        self.name()
    }

    /// Whether a target bitrate means anything here. Uncompressed audio has
    /// exactly one rate — its sample rate times its width — so `-b:a` against
    /// it is a setting that silently does nothing, and we would rather not
    /// send it at all.
    pub const fn takes_bitrate(self) -> bool {
        match self {
            Self::Aac | Self::Wmav2 => true,
            Self::PcmS16Le => false,
        }
    }
}

impl fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for AudioCodec {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|codec| codec.name().eq_ignore_ascii_case(text.trim()))
            .ok_or_else(|| FormatError::UnknownAudioCodec {
                name: text.trim().to_owned(),
            })
    }
}
