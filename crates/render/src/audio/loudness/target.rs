//! The loudness a render is asked to deliver at.

use std::fmt;
use std::str::FromStr;

/// An integrated loudness to bring a render's soundtrack to, in LUFS.
///
/// A number and not a destination. Presets named for where a video is going
/// (`reels`, `shorts`, …) are planned on top of it, each figure taken from
/// that platform's own published guidance (#977); until those studies land,
/// no figure is written here from memory.
///
/// Held between [`LoudnessTarget::QUIETEST`] and [`LoudnessTarget::LOUDEST`]:
/// nothing is delivered quieter than the first, and past the second a mix is
/// nearly all limiter. A number outside that is a typo, and it is caught where
/// it is typed rather than spent on a render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessTarget(f64);

impl LoudnessTarget {
    /// The quietest target accepted, in LUFS.
    pub const QUIETEST: f64 = -40.0;
    /// The loudest target accepted, in LUFS.
    pub const LOUDEST: f64 = -5.0;

    /// A target of `lufs`, refused outside the accepted range.
    pub fn new(lufs: f64) -> Result<Self, LoudnessTargetError> {
        if lufs > 0.0 {
            return Err(LoudnessTargetError::Positive(lufs));
        }
        if !(Self::QUIETEST..=Self::LOUDEST).contains(&lufs) {
            return Err(LoudnessTargetError::OutOfRange(lufs));
        }
        Ok(Self(lufs))
    }

    /// The target in LUFS.
    pub const fn lufs(self) -> f64 {
        self.0
    }
}

impl fmt::Display for LoudnessTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} LUFS", self.0)
    }
}

impl FromStr for LoudnessTarget {
    type Err = LoudnessTargetError;

    /// Parses `-14`, `-14.5` or `-14 LUFS`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let digits = text
            .strip_suffix("LUFS")
            .or_else(|| text.strip_suffix("lufs"))
            .unwrap_or(text);
        let lufs: f64 = digits
            .trim()
            .parse()
            .map_err(|_| LoudnessTargetError::Malformed)?;
        if !lufs.is_finite() {
            return Err(LoudnessTargetError::Malformed);
        }
        Self::new(lufs)
    }
}

/// A loudness target that cannot be delivered at.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum LoudnessTargetError {
    /// The number did not parse.
    #[error("expected a loudness in LUFS like `-14`")]
    Malformed,
    /// Loudness in LUFS is negative, so a positive number is almost always the
    /// right one with its sign left off.
    #[error("a loudness in LUFS is negative: did you mean -{0}?")]
    Positive(f64),
    /// Outside what anything is delivered at.
    #[error(
        "{0} LUFS is outside {quietest} to {loudest} LUFS, the range a delivery is made at",
        quietest = LoudnessTarget::QUIETEST,
        loudest = LoudnessTarget::LOUDEST
    )]
    OutOfRange(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_reads_with_or_without_its_unit() {
        for text in ["-14", " -14 LUFS", "-14lufs", "-14.0"] {
            assert_eq!(text.parse::<LoudnessTarget>().map(|t| t.lufs()), Ok(-14.0));
        }
    }

    #[test]
    fn a_target_out_of_reach_is_refused_where_it_is_typed() {
        assert_eq!(
            "14".parse::<LoudnessTarget>(),
            Err(LoudnessTargetError::Positive(14.0))
        );
        assert_eq!(
            "loud".parse::<LoudnessTarget>(),
            Err(LoudnessTargetError::Malformed)
        );
        assert_eq!(
            "NaN".parse::<LoudnessTarget>(),
            Err(LoudnessTargetError::Malformed)
        );
        assert_eq!(
            "-41".parse::<LoudnessTarget>(),
            Err(LoudnessTargetError::OutOfRange(-41.0))
        );
        assert_eq!(
            "-4".parse::<LoudnessTarget>(),
            Err(LoudnessTargetError::OutOfRange(-4.0))
        );
        assert!("-40".parse::<LoudnessTarget>().is_ok());
        assert!("-5".parse::<LoudnessTarget>().is_ok());
        assert!("0".parse::<LoudnessTarget>().is_err());
    }
}
