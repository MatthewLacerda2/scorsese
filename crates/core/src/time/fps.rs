//! The timeline framerate: an exact rational, never a float.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::frames::Frames;

/// The grid a project's timeline is authored against, as an exact rational —
/// `{ "num": 30, "den": 1 }` for 30, `{ "num": 30000, "den": 1001 }` for
/// 29.97.
///
/// Rational because the NTSC rates are not floats: 29.97 *is* 30000/1001, and
/// rounding it is where long-timeline drift comes from. Frame 10,000 sits at
/// exactly 333.667s and integer frames on a rational rate say so; accumulated
/// float seconds do not.
///
/// Chosen once, at project creation. Changing a project's timeline framerate
/// afterwards is a real operation (rescale the edit, or reinterpret it?) and
/// deliberately not a field edit.
///
/// Always stored in lowest terms, so `60/2` and `30/1` are one value rather
/// than two that compare unequal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "FpsWire")]
pub struct Fps {
    num: u32,
    den: u32,
}

impl Fps {
    /// 24 — film.
    pub const FILM: Self = Self { num: 24, den: 1 };
    /// 25 — PAL.
    pub const PAL: Self = Self { num: 25, den: 1 };
    /// 30 — the default grid for a new project.
    pub const THIRTY: Self = Self { num: 30, den: 1 };
    /// 60.
    pub const SIXTY: Self = Self { num: 60, den: 1 };
    /// 23.976 — film pulled down for NTSC, exactly 24000/1001.
    pub const NTSC_FILM: Self = Self {
        num: 24000,
        den: 1001,
    };
    /// 29.97 — NTSC, exactly 30000/1001.
    pub const NTSC: Self = Self {
        num: 30000,
        den: 1001,
    };

    /// A framerate from a rational. Both parts must be non-zero; the fraction
    /// is reduced, so this is the only way a valid `Fps` comes into being.
    pub fn new(num: u32, den: u32) -> Result<Self, FpsError> {
        if num == 0 || den == 0 {
            return Err(FpsError { num, den });
        }
        let divisor = gcd(num, den);
        Ok(Self {
            num: num / divisor,
            den: den / divisor,
        })
    }

    /// The numerator, in lowest terms — 30000 for 29.97.
    pub const fn num(self) -> u32 {
        self.num
    }

    /// The denominator, in lowest terms — 1001 for 29.97, 1 for a whole rate.
    pub const fn den(self) -> u32 {
        self.den
    }

    /// The rate as a float — for display, and for the ffmpeg arguments that
    /// take one. Never for timeline arithmetic.
    pub fn as_f64(self) -> f64 {
        f64::from(self.num) / f64::from(self.den)
    }

    /// Where a frame on this grid falls in wall-clock time.
    pub fn seconds(self, frames: Frames) -> f64 {
        frames.get() as f64 * f64::from(self.den) / f64::from(self.num)
    }

    /// The frame on this grid nearest a wall-clock time. Anything at or below
    /// zero — and NaN — lands on frame 0, because the timeline starts there.
    pub fn frames(self, seconds: f64) -> Frames {
        if seconds.is_nan() || seconds <= 0.0 {
            return Frames::ZERO;
        }
        Frames((seconds * self.as_f64()).round() as u64)
    }

    /// Re-times a frame count from another grid onto this one, picking the
    /// **nearest** frame in wall-clock time.
    ///
    /// This is the conform rule, and it is deliberately the honest, simple
    /// one: no interpolation, no invented in-between frames. A 24fps source
    /// on a 30fps timeline repeats source frames in a 2:3 pattern; a render
    /// at an output rate other than the timeline's conforms the same way,
    /// with the timeline grid staying authoritative for what is on screen
    /// when.
    pub fn conform(self, frames: Frames, from: Fps) -> Frames {
        // Exact: frames × (from.den / from.num) seconds × (num / den) frames.
        // Both parts of both rates are non-zero, so nothing here divides by
        // zero and nothing rounds until the last step.
        let numerator = u128::from(frames.get()) * u128::from(from.den) * u128::from(self.num);
        let denominator = u128::from(from.num) * u128::from(self.den);
        let rounded = (numerator + denominator / 2) / denominator;
        Frames(u64::try_from(rounded).unwrap_or(u64::MAX))
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self::THIRTY
    }
}

impl fmt::Display for Fps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl FromStr for Fps {
    type Err = FpsParseError;

    /// Parses `30` or `30000/1001`. A decimal is refused on purpose: 29.97 is
    /// not a framerate, 30000/1001 is, and accepting the first would put the
    /// rounding this type exists to prevent back at the command line.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (num, den) = match text.split_once('/') {
            Some((num, den)) => (num.trim(), den.trim()),
            None => (text.trim(), "1"),
        };
        let parse = |part: &str| part.parse::<u32>().map_err(|_| FpsParseError::Malformed);
        Ok(Self::new(parse(num)?, parse(den)?)?)
    }
}

/// The wire shape, so a framerate is validated as it is read rather than
/// later: everything else in the document is measured against this grid, and
/// there is nothing useful to say about a timeline whose grid is undefined.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FpsWire {
    num: u32,
    den: u32,
}

impl TryFrom<FpsWire> for Fps {
    type Error = FpsError;

    fn try_from(wire: FpsWire) -> Result<Self, Self::Error> {
        Self::new(wire.num, wire.den)
    }
}

/// A rational that is not a framerate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("framerate {num}/{den} is unusable: both parts must be non-zero")]
pub struct FpsError {
    /// The numerator as offered, before reduction.
    pub num: u32,
    /// The denominator as offered, before reduction.
    pub den: u32,
}

/// Text that is not a framerate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FpsParseError {
    /// Not two integers, or not integers at all — `29.97` lands here, which is
    /// the point.
    #[error("expected a framerate like `30` or `30000/1001`")]
    Malformed,
    /// Read as a fraction, but not one a timeline can be measured on.
    #[error(transparent)]
    Unusable(#[from] FpsError),
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}
