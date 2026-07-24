//! Timeline time.

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A point in time, or a span of it, in seconds.
///
/// The timeline is measured in **seconds, not frames**. That follows from a
/// decided architecture rule: frame rate is a *render setting*, chosen per
/// render, so it is not known when the project is authored. A frame-based
/// timeline would silently bind a project to one fps.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seconds(pub f64);

impl Seconds {
    pub const ZERO: Self = Self(0.0);

    /// Seconds as a plain `f64`.
    pub fn get(self) -> f64 {
        self.0
    }

    /// True when this value is usable as a time: finite and not negative.
    /// Checked by validation rather than enforced at construction, so a
    /// project with several bad times reports all of them at once.
    pub fn is_valid(self) -> bool {
        self.0.is_finite() && self.0 >= 0.0
    }

    /// Total ordering, safe because NaN sorts consistently rather than
    /// poisoning comparisons.
    pub fn cmp_total(self, other: Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::ops::Add for Seconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl From<f64> for Seconds {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
