//! What a render produces: how big, how fast, how heavy.
//!
//! These are chosen **per render** and never stored in the project. The
//! project owns the timeline grid (`timeline_fps`), which says what is on
//! screen when; a render conforms from that grid to whatever is asked for
//! here.
//!
//! Aspect ratio is deliberately not a separate setting: it is whatever
//! [`Resolution`] says. Two independent settings could contradict each other,
//! and the only thing that could reconcile them — non-square pixels — is a
//! legacy this project has no reason to carry. Source material of a different
//! shape is letterboxed to fit, never stretched.

use std::fmt;
use std::str::FromStr;

use scorsese_core::Fps;

/// The pixel dimensions of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    width: u32,
    height: u32,
}

impl Resolution {
    /// 1920×1080 — the default when nobody says otherwise.
    pub const HD: Self = Self {
        width: 1920,
        height: 1080,
    };

    /// Both dimensions must be non-zero and even.
    ///
    /// Even because the encoder writes 4:2:0 chroma, where an odd dimension
    /// has no defined half. Refusing here, before anything spawns, beats an
    /// ffmpeg error surfacing from inside a render that already started.
    pub fn new(width: u32, height: u32) -> Result<Self, ResolutionError> {
        if width == 0 || height == 0 {
            return Err(ResolutionError::Zero);
        }
        if width % 2 == 1 || height % 2 == 1 {
            return Err(ResolutionError::Odd { width, height });
        }
        Ok(Self { width, height })
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    /// How many pixels one frame holds.
    pub const fn pixels(self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Width over height — derived, never stored, so it cannot disagree with
    /// the dimensions it describes.
    pub fn aspect(self) -> f64 {
        f64::from(self.width) / f64::from(self.height)
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl FromStr for Resolution {
    type Err = ResolutionError;

    /// Parses `1920x1080`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (width, height) = text
            .split_once(['x', 'X'])
            .ok_or(ResolutionError::Malformed)?;
        let parse = |part: &str| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| ResolutionError::Malformed)
        };
        Self::new(parse(width)?, parse(height)?)
    }
}

/// Dimensions that cannot be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    #[error("expected a resolution like `1920x1080`")]
    Malformed,
    #[error("a resolution cannot have a zero dimension")]
    Zero,
    #[error("{width}x{height} has an odd dimension; 4:2:0 video needs both to be even")]
    Odd { width: u32, height: u32 },
}

/// A target video bitrate, in bits per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bitrate(u64);

impl Bitrate {
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
    #[error("expected a bitrate like `8M`, `800k`, or a count of bits per second")]
    Malformed,
    #[error("a bitrate of zero would encode nothing")]
    Zero,
}

/// Everything a render needs to know that the project does not decide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSettings {
    pub resolution: Resolution,
    /// The output framerate. Rendering at a rate other than the project's
    /// timeline grid conforms from it — nearest frame, no interpolation.
    pub fps: Fps,
    /// Target bitrate. `None` asks the encoder for constant quality instead,
    /// which is the better answer when nobody has a file-size budget.
    pub bitrate: Option<Bitrate>,
}

impl RenderSettings {
    pub fn new(resolution: Resolution, fps: Fps) -> Self {
        Self {
            resolution,
            fps,
            bitrate: None,
        }
    }

    pub fn with_bitrate(self, bitrate: Option<Bitrate>) -> Self {
        Self { bitrate, ..self }
    }
}
