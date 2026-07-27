//! Naming an instant of a render, in either of the two units it has.
//!
//! A description prints seconds and frames side by side because a reader thinks
//! in one and the document is authored in the other. Asking for a still has the
//! same problem from the other end, so it takes either: `2.5s` is what someone
//! watching says, `75` is what the `project.json` in front of them says, and
//! making the caller convert is how an off-by-one gets introduced by the person
//! trying to find one.

use std::fmt;
use std::str::FromStr;

use scorsese_core::Frames;

use super::Description;

/// An instant of a render, as somebody asked for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cue {
    /// A frame of the **timeline**, counted the way the document counts.
    Frame(Frames),
    /// A wall-clock second of the timeline.
    Seconds(f64),
}

impl Cue {
    /// Which frame of the delivered file this instant lands on.
    ///
    /// Both units go through the same conversion — a frame becomes a time and a
    /// time becomes an output frame — so a render at an output rate other than
    /// the timeline's answers both consistently, and neither is the special
    /// case.
    pub fn out_frame(self, description: &Description) -> u64 {
        let seconds = match self {
            Self::Frame(frame) => description.timeline_fps.seconds(frame),
            Self::Seconds(seconds) => seconds,
        };
        description.out_frame_at(seconds)
    }
}

impl FromStr for Cue {
    type Err = CueError;

    /// `2.5s` is a time, `75` is a frame. The suffix is what distinguishes
    /// them, and a bare decimal is refused rather than guessed at: `2.5` as a
    /// frame number is not a thing, and silently reading it as seconds would
    /// make `2` and `2.5` mean instants 2.5 seconds apart.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        match text.strip_suffix('s') {
            Some(seconds) => seconds
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
                .map(Self::Seconds)
                .ok_or(CueError::Malformed),
            None => text
                .parse::<u64>()
                .map(|frame| Self::Frame(Frames(frame)))
                .map_err(|_| CueError::Malformed),
        }
    }
}

impl fmt::Display for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(frame) => write!(f, "{}", frame.get()),
            Self::Seconds(seconds) => write!(f, "{seconds}s"),
        }
    }
}

/// Text that does not name an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CueError {
    /// Neither a whole frame number nor a time in seconds.
    #[error("expected a timeline frame like `75` or a time like `2.5s`")]
    Malformed,
}
