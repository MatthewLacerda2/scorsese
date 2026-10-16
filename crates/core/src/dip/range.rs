//! A stretch of narration asked for in seconds, checked once for every caller.
//!
//! The command line and the MCP server both take bare stretches of the
//! timeline (`--range 12.5:15`, `ranges: [[12.5, 15]]`), and a stretch that
//! starts before zero or runs backwards is a mistake to say out loud — put on
//! the frame grid as it is, it would be clamped to something nobody asked for
//! and ducked under without a word (#1288).

use std::ops::Range;

use crate::time::{Fps, Frames};

use super::Under;

/// Why a stretch given in seconds is not one.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum DuckRangeError {
    /// An end that is negative, infinite or not a number.
    #[error("{end} in the range {from} to {to} is not a time in seconds")]
    NotATime {
        /// The end at fault.
        end: f64,
        /// Where the range starts, as given.
        from: f64,
        /// Where the range ends, as given.
        to: f64,
    },
    /// An end at or before the start: nothing to duck under.
    #[error("the range {from} to {to} ends before it starts")]
    Backwards {
        /// Where the range starts, as given.
        from: f64,
        /// Where the range ends, as given.
        to: f64,
    },
}

impl DuckRangeError {
    /// Refuses `from..to` in seconds unless both ends are times from zero up
    /// and it runs forward.
    pub fn check(from: f64, to: f64) -> Result<(), Self> {
        for end in [from, to] {
            if !(end.is_finite() && end >= 0.0) {
                return Err(Self::NotATime { end, from, to });
            }
        }
        if to <= from {
            return Err(Self::Backwards { from, to });
        }
        Ok(())
    }
}

impl Under {
    /// `from..to` in seconds as a stretch of `grid`'s frames, for
    /// [`Under::ranges`] — refused rather than clamped when it is not one.
    pub fn seconds(grid: Fps, from: f64, to: f64) -> Result<Range<Frames>, DuckRangeError> {
        DuckRangeError::check(from, to)?;
        Ok(grid.frames(from)..grid.frames(to))
    }
}
