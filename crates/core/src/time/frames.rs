//! Positions and spans on the timeline grid.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A point on the timeline, or a span of it, counted in **whole frames** on
/// the project's [`crate::Fps`] grid.
///
/// Integer, so a cut is never ambiguous: when one clip ends at frame 240 and
/// the next starts at 240, frame 240 belongs to the second and no rounding
/// gets a say. Unsigned, so a time before the start of the timeline cannot be
/// written down at all — and a negative or fractional value in `project.json`
/// fails to parse rather than being quietly rounded into place, which is the
/// whole reason the timeline is not measured in seconds.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Frames(pub u64);

impl Frames {
    /// The first frame of the timeline, and the default for every time a
    /// document leaves out.
    pub const ZERO: Self = Self(0);

    /// The frame count as a plain `u64`.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::ops::Add for Frames {
    type Output = Self;

    /// Saturating, so a nonsense document (a clip starting at `u64::MAX` and
    /// running for a day) is a validation problem rather than a panic.
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl fmt::Display for Frames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}f", self.0)
    }
}

impl From<u64> for Frames {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
