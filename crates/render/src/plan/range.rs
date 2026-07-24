//! Rendering only part of the timeline.

use std::fmt;
use std::str::FromStr;

use scorsese_core::Frames;

/// A slice of the timeline to render: `30:120` covers frames 30 up to but not
/// including 120. `30:` runs to the end of the timeline, `:120` from its
/// start.
///
/// Frames, not seconds, because the timeline is frames — asking for `1.5s`
/// would reintroduce, at the command line, exactly the rounding the frame grid
/// exists to prevent. End-exclusive for the same reason clip ends are: the
/// frame two ranges share belongs to the later one, with nothing to decide.
///
/// This is the cheap iteration loop. Re-rendering four seconds to check one
/// cut costs four seconds of encoding, not the whole timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRange {
    start: Frames,
    end: Option<Frames>,
}

impl FrameRange {
    /// The whole timeline.
    pub const ALL: Self = Self {
        start: Frames::ZERO,
        end: None,
    };

    /// A range from `start`, ending at `end` or running to the timeline's own
    /// end when that is `None`.
    pub fn new(start: Frames, end: Option<Frames>) -> Result<Self, FrameRangeError> {
        if let Some(end) = end
            && end <= start
        {
            return Err(FrameRangeError::Backwards { start, end });
        }
        Ok(Self { start, end })
    }

    pub const fn start(self) -> Frames {
        self.start
    }

    pub const fn end(self) -> Option<Frames> {
        self.end
    }
}

impl Default for FrameRange {
    fn default() -> Self {
        Self::ALL
    }
}

impl fmt::Display for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.end {
            Some(end) => write!(f, "{}:{}", self.start.get(), end.get()),
            None => write!(f, "{}:", self.start.get()),
        }
    }
}

impl FromStr for FrameRange {
    type Err = FrameRangeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (start, end) = text.split_once(':').ok_or(FrameRangeError::Malformed)?;
        let part = |text: &str| -> Result<Option<Frames>, FrameRangeError> {
            let text = text.trim();
            if text.is_empty() {
                return Ok(None);
            }
            text.parse::<u64>()
                .map(|frame| Some(Frames(frame)))
                .map_err(|_| FrameRangeError::Malformed)
        };
        Self::new(part(start)?.unwrap_or(Frames::ZERO), part(end)?)
    }
}

/// Text that is not a frame range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameRangeError {
    #[error("expected a frame range like `30:120`, `30:`, or `:120`")]
    Malformed,
    #[error("frame range {}:{} ends before it begins", start.get(), end.get())]
    Backwards { start: Frames, end: Frames },
}
