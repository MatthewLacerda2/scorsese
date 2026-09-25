//! The encoders scorsese will ask ffmpeg for.
//!
//! Every one of these is either already required by the default path
//! (`libx264`) or built into ffmpeg itself with no external library behind it
//! (`mpeg4`, `wmv2`, `aac`, `pcm_s16le`, `wmav2`) — with **one** exception,
//! `libmp3lame`, taken on purpose. Which ffmpeg we are talking to is a shipping
//! decision — a distro build in dev and CI, a bundled sidecar in a shipped
//! build — so a codec list that needs a particular build is a list we cannot
//! stand behind unless the render asks the build before it spends anything.
//! [`AudioCodec::library`] is that question, and mp3 is why it exists: it is
//! the format people mean by "an audio file", and the user asked for it by
//! name (#505).

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
    /// MPEG-1 Audio Layer III, via `libmp3lame`. What "an audio file" means to
    /// almost everyone, and the one codec here an ffmpeg build can be without
    /// — see [`AudioCodec::library`].
    Mp3,
}

impl AudioCodec {
    /// Every codec, in the order they are listed to a reader.
    pub const ALL: [Self; 4] = [Self::Aac, Self::PcmS16Le, Self::Wmav2, Self::Mp3];

    /// The name this is written and parsed as, on the command line and in
    /// `docs/output-formats.md`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::PcmS16Le => "pcm_s16le",
            Self::Wmav2 => "wmav2",
            Self::Mp3 => "mp3",
        }
    }

    /// The ffmpeg encoder to ask for, which is the codec's own name for every
    /// encoder built into ffmpeg and the library's for the one that is not.
    pub const fn encoder(self) -> &'static str {
        match self.library() {
            Some(library) => library,
            None => self.name(),
        }
    }

    /// The external library this codec is encoded by, when it is one — an
    /// encoder a particular ffmpeg build may simply not have.
    ///
    /// A render in such a codec asks the ffmpeg on hand whether it has the
    /// encoder before anything is mixed or encoded, and refuses with this name
    /// when it does not, rather than failing part way into an encode with
    /// ffmpeg's own "unknown encoder".
    pub const fn library(self) -> Option<&'static str> {
        match self {
            Self::Mp3 => Some("libmp3lame"),
            Self::Aac | Self::PcmS16Le | Self::Wmav2 => None,
        }
    }

    /// Whether this encoder reconstructs a *different* waveform from the one it
    /// was given, rather than the same samples back.
    ///
    /// What decides whether a render has to leave the encoder headroom
    /// (`crate::audio::headroom`): a lossy codec rebuilds the signal from
    /// what it kept of the spectrum, and the rebuilt peaks land above the
    /// originals by an amount that depends on the material. Asked of the codec
    /// rather than of any one of them by name, so an audio-only delivery in
    /// another lossy codec inherits the same protection without anyone
    /// remembering to extend a match on `aac`.
    pub const fn is_lossy(self) -> bool {
        match self {
            Self::Aac | Self::Wmav2 | Self::Mp3 => true,
            Self::PcmS16Le => false,
        }
    }

    /// Whether a target bitrate means anything here. Uncompressed audio has
    /// exactly one rate — its sample rate times its width — so `-b:a` against
    /// it is a setting that silently does nothing, and we would rather not
    /// send it at all.
    pub const fn takes_bitrate(self) -> bool {
        match self {
            Self::Aac | Self::Wmav2 | Self::Mp3 => true,
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
