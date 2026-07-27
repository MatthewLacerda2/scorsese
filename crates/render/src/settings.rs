//! What a render produces: how big, how fast, how heavy.
//!
//! These are chosen **per render** and never stored in the project. The
//! project owns the timeline grid (`timeline_fps`), which says what is on
//! screen when; a render conforms from that grid to whatever is asked for
//! here.
//!
//! Audio is chosen here for the same reason picture is: a sample rate is a
//! property of the file being delivered, not of the edit. The mix is produced
//! at [`SampleRate`] whatever the sources were recorded at.
//!
//! Aspect ratio is deliberately not a separate setting: it is whatever
//! [`Resolution`] says. Two independent settings could contradict each other,
//! and the only thing that could reconcile them — non-square pixels — is a
//! legacy this project has no reason to carry. Source material of a different
//! shape meets the raster the way its clip's [`scorsese_core::Fit`] says —
//! letterboxed, cropped, or left at its own size — and is never stretched.

use std::fmt;
use std::str::FromStr;

use scorsese_core::Fps;

pub use scorsese_compositor::{Resolution, ResolutionError};

/// A target video bitrate, in bits per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bitrate(u64);

impl Bitrate {
    /// The rate as a plain number, for callers doing arithmetic on it rather
    /// than handing it to an encoder.
    pub const fn bits_per_second(self) -> u64 {
        self.0
    }

    /// The value as ffmpeg's `-b:v` takes it.
    pub fn ffmpeg_value(self) -> String {
        self.0.to_string()
    }
}

impl FromStr for Bitrate {
    type Err = BitrateError;

    /// Parses `8M`, `800k`, or a plain count of bits per second.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (digits, scale) = match text.strip_suffix(['M', 'm']) {
            Some(digits) => (digits, 1_000_000),
            None => match text.strip_suffix(['k', 'K']) {
                Some(digits) => (digits, 1_000),
                None => (text, 1),
            },
        };
        let value: u64 = digits.trim().parse().map_err(|_| BitrateError::Malformed)?;
        let bits = value.checked_mul(scale).ok_or(BitrateError::Malformed)?;
        if bits == 0 {
            return Err(BitrateError::Zero);
        }
        Ok(Self(bits))
    }
}

impl fmt::Display for Bitrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            bits if bits >= 1_000_000 => write!(f, "{:.1} Mbit/s", bits as f64 / 1e6),
            bits if bits >= 1_000 => write!(f, "{} kbit/s", bits / 1_000),
            bits => write!(f, "{bits} bit/s"),
        }
    }
}

/// Text that is not a bitrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BitrateError {
    /// The digits, or the `M`/`k` suffix, did not parse.
    #[error("expected a bitrate like `8M`, `800k`, or a count of bits per second")]
    Malformed,
    /// Refused rather than passed on: ffmpeg treats `-b:v 0` as a licence to
    /// produce garbage.
    #[error("a bitrate of zero would encode nothing")]
    Zero,
}

/// How many samples per second a render mixes and encodes at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRate(u32);

impl SampleRate {
    /// 48 kHz — what video production runs at, and what every encoder we hand
    /// audio to takes without resampling.
    pub const DEFAULT: Self = Self(48_000);

    /// A rate in Hz. Zero is refused: every sample-count calculation in the
    /// mixer divides by this.
    pub fn new(hz: u32) -> Result<Self, SampleRateError> {
        if hz == 0 {
            return Err(SampleRateError::Zero);
        }
        Ok(Self(hz))
    }

    /// The rate in Hz, which is what both ffmpeg and the mixer's sample counts
    /// want.
    pub const fn hz(self) -> u32 {
        self.0
    }
}

impl Default for SampleRate {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

impl FromStr for SampleRate {
    type Err = SampleRateError;

    /// Parses `48000` or `48k`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (digits, scale) = match text.strip_suffix(['k', 'K']) {
            Some(digits) => (digits, 1_000),
            None => (text, 1),
        };
        let value: u32 = digits
            .trim()
            .parse()
            .map_err(|_| SampleRateError::Malformed)?;
        Self::new(value.saturating_mul(scale))
    }
}

/// Text that is not a sample rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SampleRateError {
    /// The digits, or the `k` suffix, did not parse.
    #[error("expected a sample rate like `48000` or `48k`")]
    Malformed,
    /// Caught at construction because every sample-count calculation in the
    /// mixer divides by the rate.
    #[error("a sample rate of zero has no samples in it")]
    Zero,
}

/// Everything a render needs to know that the project does not decide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSettings {
    /// The raster every frame is composited into, and — since there is no
    /// separate aspect setting — the shape of the delivered file.
    pub resolution: Resolution,
    /// The output framerate. Rendering at a rate other than the project's
    /// timeline grid conforms from it — nearest frame, no interpolation.
    pub fps: Fps,
    /// Target video bitrate. `None` asks the encoder for constant quality
    /// instead, which is the better answer when nobody has a file-size budget.
    pub bitrate: Option<Bitrate>,
    /// What the mix is produced at. Sources of any rate are resampled to it on
    /// the way in, so this is the one rate the mixer ever works in.
    pub sample_rate: SampleRate,
    /// Target audio bitrate. `None` leaves the encoder on its own default,
    /// which for AAC is already transparent for speech and music beds.
    pub audio_bitrate: Option<Bitrate>,
}

impl RenderSettings {
    /// The two settings with no sensible default, everything else left at one:
    /// constant quality for picture, 48 kHz and the encoder's own choice for
    /// sound.
    pub fn new(resolution: Resolution, fps: Fps) -> Self {
        Self {
            resolution,
            fps,
            bitrate: None,
            sample_rate: SampleRate::DEFAULT,
            audio_bitrate: None,
        }
    }

    /// Puts the picture on a file-size budget. `None` keeps constant quality.
    pub fn with_bitrate(self, bitrate: Option<Bitrate>) -> Self {
        Self { bitrate, ..self }
    }

    /// Sets both audio settings together, because a rate and a bitrate are
    /// chosen against each other — 128k means something different at 48 kHz
    /// than at 8 kHz.
    pub fn with_audio(self, sample_rate: SampleRate, audio_bitrate: Option<Bitrate>) -> Self {
        Self {
            sample_rate,
            audio_bitrate,
            ..self
        }
    }
}
