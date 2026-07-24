//! What a render did, and what it wants you to know about it.

use std::fmt;

use scorsese_core::{Fps, Frames};

use crate::settings::Resolution;

/// Something worth saying about a render that is not a reason to refuse it.
///
/// These are signals, not gates: every one of them describes output that was
/// produced successfully but may not be what the author meant. A render that
/// quietly drops the music track is the failure mode this type exists to
/// prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// The project has audio, and this pipeline does not mix it yet.
    AudioNotMixed { tracks: usize },
    /// The requested range ran past the end of the timeline.
    RangeClamped { asked: Frames, timeline_end: Frames },
    /// A clip outlasts its own source media; the remainder rendered black.
    ClipRanShort { clip: String, missing: u64 },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AudioNotMixed { tracks } => write!(
                f,
                "{tracks} audio track(s) were not mixed in — this render is silent"
            ),
            Self::RangeClamped {
                asked,
                timeline_end,
            } => write!(
                f,
                "the range asked for frame {} but the timeline ends at {timeline_end}",
                asked.get()
            ),
            Self::ClipRanShort { clip, missing } => write!(
                f,
                "clip `{clip}` outlasts its source by {missing} frame(s), which rendered black"
            ),
        }
    }
}

/// The outcome of a completed render.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderReport {
    /// How many frames were written.
    pub frames: u64,
    pub fps: Fps,
    pub resolution: Resolution,
    pub notes: Vec<Note>,
}

impl RenderReport {
    /// How long the output runs, in wall-clock seconds.
    pub fn seconds(&self) -> f64 {
        self.fps.seconds(Frames(self.frames))
    }
}
