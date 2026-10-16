//! The shape of a project's frame (#1278): its aspect ratio, chosen when the
//! project is made and changeable later, and the arithmetic every surface
//! sizes a raster with.
//!
//! **A ratio, not a pixel size.** How many pixels a render has is still chosen
//! per render — a preview is small, a delivery is 1080 lines — but what shape
//! they are arranged in is a fact about the edit: a title laid out for 9:16 is
//! a different title at 16:9. Before this lived here every surface guessed the
//! shape, and each guessed differently (#1116, #1199, #1145).
//!
//! **One way to round.** A side worked out from a ratio is rounded to the
//! nearest even number, because the encoder's 4:2:0 chroma has no defined half
//! of an odd one, and [`even`] is the only place that rounding is written.

mod refit;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use refit::{Refit, picture_shape, refit};

use crate::asset::Aspect;

/// How far from square a frame may be, either way up. No delivery is narrower
/// than this, and a raster sized from a sliver would be a sliver.
pub const MOST: f64 = 4.0;

/// The pixels on a delivery's shorter side: 1080, so 16:9 delivers at
/// 1920x1080 and 9:16 at 1080x1920.
pub const DELIVERY_LINES: u32 = 1080;

/// How close two shapes must be to be one shape: a hundredth of the ratio.
/// A 1920x1088 encode is 16:9 with padding, not a shape of its own.
const SAME: f64 = 0.01;

/// The shapes a measured size is read as when it is within [`SAME`] of one,
/// so a probed 1366x768 or 1920x1088 is written as the 16:9 it was meant as.
const COMMON: [(u32, u32); 11] = [
    (16, 9),
    (9, 16),
    (1, 1),
    (4, 5),
    (5, 4),
    (4, 3),
    (3, 4),
    (3, 2),
    (2, 3),
    (21, 9),
    (9, 21),
];

/// A frame's shape, `across:down` in lowest terms — `16:9`, `9:16`, `4:5`.
///
/// Spelled in `project.json` the way a generated asset's `aspect` is, as one
/// string. Always within [`MOST`] of square, so anything sized from it is a
/// picture rather than a ribbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ratio {
    across: u32,
    down: u32,
}

impl Ratio {
    /// Landscape, 16:9: what a project is when nobody says otherwise.
    pub const WIDE: Self = Self {
        across: 16,
        down: 9,
    };

    /// Upright, 9:16: every vertical feed.
    pub const TALL: Self = Self {
        across: 9,
        down: 16,
    };

    /// `across:down`, reduced to lowest terms, or why it cannot be a frame.
    pub fn new(across: u32, down: u32) -> Result<Self, RatioError> {
        if across == 0 || down == 0 {
            return Err(RatioError::Zero);
        }
        let ratio = f64::from(across) / f64::from(down);
        if !(1.0 / MOST..=MOST).contains(&ratio) {
            return Err(RatioError::TooFar { across, down });
        }
        let common = gcd(across, down);
        Ok(Self {
            across: across / common,
            down: down / common,
        })
    }

    /// The shape of a `width`×`height` picture, read as the common shape it is
    /// within a hundredth of when there is one — `None` for a size too far
    /// from square to be a frame (a 40x1000 ribbon).
    pub fn of_size(width: u32, height: u32) -> Option<Self> {
        let measured = Self::new(width, height).ok()?;
        let near = COMMON.into_iter().find(|&(across, down)| {
            let common = f64::from(across) / f64::from(down);
            (measured.value() / common - 1.0).abs() < SAME
        });
        Some(near.map_or(measured, |(across, down)| Self { across, down }))
    }

    /// Parts across.
    pub const fn across(self) -> u32 {
        self.across
    }

    /// Parts down.
    pub const fn down(self) -> u32 {
        self.down
    }

    /// Width over height.
    pub fn value(self) -> f64 {
        f64::from(self.across) / f64::from(self.down)
    }

    /// Whether a picture of shape `other` fills a frame of this shape edge to
    /// edge — the same shape, to within a hundredth.
    pub fn matches(self, other: Self) -> bool {
        (self.value() / other.value() - 1.0).abs() < SAME
    }

    /// Whether it is taller than it is wide.
    pub fn is_upright(self) -> bool {
        self.down > self.across
    }

    /// The size a render in this shape is delivered at when nobody names one,
    /// `(width, height)`: [`DELIVERY_LINES`] on the shorter side.
    pub fn delivery(self) -> (u32, u32) {
        let lines = f64::from(DELIVERY_LINES);
        if self.is_upright() {
            (DELIVERY_LINES, even(lines / self.value()))
        } else {
            (even(lines * self.value()), DELIVERY_LINES)
        }
    }

    /// The raster of about `pixels` pixels in this shape, `(width, height)`,
    /// each side [`even`].
    pub fn spend(self, pixels: u64) -> (u32, u32) {
        // A budget of a few million pixels at most is exact in an f64.
        let width = (pixels as f64 * self.value()).sqrt();
        (even(width), even(width / self.value()))
    }
}

impl Default for Ratio {
    fn default() -> Self {
        Self::WIDE
    }
}

impl From<Aspect> for Ratio {
    fn from(aspect: Aspect) -> Self {
        match aspect {
            Aspect::Wide => Self::WIDE,
            Aspect::Tall => Self::TALL,
        }
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.across, self.down)
    }
}

impl FromStr for Ratio {
    type Err = RatioError;

    /// Parses `9:16`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (across, down) = text.split_once(':').ok_or(RatioError::Malformed)?;
        let term = |part: &str| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| RatioError::Malformed)
        };
        Self::new(term(across)?, term(down)?)
    }
}

impl TryFrom<String> for Ratio {
    type Error = RatioError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Ratio> for String {
    fn from(ratio: Ratio) -> Self {
        ratio.to_string()
    }
}

/// Why a text or two numbers are not a frame's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RatioError {
    /// Not `across:down` in whole numbers.
    #[error("expected an aspect like `9:16` or `16:9`")]
    Malformed,
    /// A term of zero.
    #[error("an aspect cannot have a zero term")]
    Zero,
    /// Further from square than [`MOST`].
    #[error("{across}:{down} is further from square than 4:1 or 1:4, which no frame is")]
    TooFar {
        /// Parts across, as written.
        across: u32,
        /// Parts down, as written.
        down: u32,
    },
}

/// `side` rounded to the nearest even number of pixels, and never below two:
/// the one rounding every raster sized from a ratio goes through.
pub fn even(side: f64) -> u32 {
    // A positive side of a few thousand pixels at most.
    let halves = (side / 2.0).round() as u32;
    halves.max(1) * 2
}

/// The greatest common divisor, for lowest terms.
const fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests;
