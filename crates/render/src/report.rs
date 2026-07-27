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
    /// Audio carries on past the last picture, and was cut off there.
    AudioTrimmed {
        /// Where the soundtrack would have run to.
        audio_end: Frames,
        /// Where the last picture ends, and so where the file does.
        timeline_end: Frames,
    },
    /// The requested range ran past the end of the timeline.
    RangeClamped {
        /// The end frame that was asked for.
        asked: Frames,
        /// The end frame it was cut back to.
        timeline_end: Frames,
    },
    /// A clip outlasts its own source media; the remainder rendered black.
    ClipRanShort {
        /// The clip that ran out of pictures.
        clip: String,
        /// How many output frames came up black.
        missing: u64,
    },
    /// An audio clip outlasts its own source; the remainder is silence.
    AudioRanShort {
        /// The clip that ran out of sound.
        clip: String,
        /// How much silence was substituted, in milliseconds.
        missing_ms: u64,
    },
    /// A video clip's own sound was left out of the mix, because whether it
    /// has any could not be established.
    ClipAudioSkipped {
        /// The clip mixed without its own sound.
        clip: String,
        /// Why the probe could not answer.
        reason: String,
    },
    /// A keyframe track animates a property nothing in this build resolves, so
    /// it will never do anything.
    UnknownProperty {
        /// The clip carrying the track.
        clip: String,
        /// The property path nothing reads.
        property: String,
        /// The nearest property that does exist, when there is one.
        did_you_mean: Option<&'static str>,
    },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AudioTrimmed {
                audio_end,
                timeline_end,
            } => write!(
                f,
                "audio runs to frame {audio_end} but the last picture ends at \
                 {timeline_end}, so the tail was cut"
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
            Self::AudioRanShort { clip, missing_ms } => write!(
                f,
                "clip `{clip}` outlasts its source by {missing_ms}ms, which is silent"
            ),
            Self::ClipAudioSkipped { clip, reason } => write!(
                f,
                "clip `{clip}` was mixed without its own sound, because its media \
                 could not be probed for one: {reason}"
            ),
            Self::UnknownProperty {
                clip,
                property,
                did_you_mean,
            } => {
                write!(f, "clip `{clip}`: nothing animates `{property}`")?;
                match did_you_mean {
                    Some(known) => write!(f, " — did you mean `{known}`?"),
                    None => Ok(()),
                }
            }
        }
    }
}

/// The outcome of a completed render.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderReport {
    /// How many frames were written.
    pub frames: u64,
    /// The rate they were written at, which with [`RenderReport::frames`] is
    /// what makes a running time.
    pub fps: Fps,
    /// The raster they were written at — the settings as honoured, not as
    /// asked for.
    pub resolution: Resolution,
    /// How much soundtrack the output carries. `None` means the render has no
    /// audio stream at all, which is not the same as a stream of silence.
    pub seconds_of_audio: Option<f64>,
    /// Everything the render wants a second look at. Empty is the good case;
    /// a caller that ignores this is the failure mode [`Note`] exists for.
    pub notes: Vec<Note>,
}

impl RenderReport {
    /// How long the output runs, in wall-clock seconds.
    pub fn seconds(&self) -> f64 {
        self.fps.seconds(Frames(self.frames))
    }
}
