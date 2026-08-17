//! What a render did, and what it wants you to know about it.

use std::fmt;

use scorsese_core::{Fps, Frames};

use crate::audio::SoundLevels;
use crate::describe::Description;
use crate::settings::Resolution;

/// Something worth saying about a render that is not a reason to refuse it.
///
/// These are signals, not gates: every one of them describes output that was
/// produced successfully but may not be what the author meant. A render that
/// quietly drops the music track is the failure mode this type exists to
/// prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// An attached arrow whose clip is not on screen in this stretch, so there
    /// was nothing to point at and the arrow was left out.
    ///
    /// Worth a note rather than silence: an arrow that simply does not appear
    /// looks exactly like one that failed to draw, and the usual cause is a
    /// timing mistake — the arrow's clip outlasting the box's, or starting
    /// before it.
    ArrowUnattached {
        /// The arrow's clip.
        clip: String,
    },

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
    ///
    /// Validation refuses this wherever the document knows enough to see it
    /// coming — an asset with a measured length bounds the clips that show it.
    /// What is left for the render to find is the rest: an asset nobody probed,
    /// and a file that changed on disk since somebody did. Both are cases where
    /// the document was honest and the media was not what it said, which is
    /// exactly what a note is for.
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
    /// An asset the project believes was generated has no file on disk, so
    /// something stood in for it and the render carried on.
    ///
    /// A warning rather than a refusal because the media can be made again —
    /// `scorsese generate` re-bills exactly this asset — and a preview cut with
    /// one card in it is worth more than no preview at all. Ignoring it means
    /// delivering a film with a gray card where a shot should be.
    GeneratedMissing {
        /// The clip that came up empty.
        clip: String,
        /// The asset whose file is not where the table says.
        asset: String,
        /// What was put there instead.
        stood_in: StandIn,
    },
    /// An icon asset names a symbol this build does not ship, so its layer came
    /// out empty.
    ///
    /// A note rather than a refusal for [`Note::GeneratedMissing`]'s reason: one
    /// mistyped string should not cost a whole preview. Worth saying loudly all
    /// the same, because an empty layer is invisible to anything counting
    /// frames — `scorsese check` reports the same fault as a *problem*, before
    /// an encode is ever started, and names the near matches with it.
    UnknownIcon {
        /// The clip whose layer came out empty.
        clip: String,
        /// The asset naming it.
        asset: String,
        /// The name as authored.
        named: String,
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

/// What a render put where media should have been.
///
/// Both halves of a prompt clip, named because the two are not the same
/// finding: a card is visible to whoever watches the result, silence is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandIn {
    /// A slug card, on the picture.
    SlugCard,
    /// Nothing, in the mix.
    Silence,
}

impl fmt::Display for StandIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SlugCard => "a slug card",
            Self::Silence => "silence",
        })
    }
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
            Self::ArrowUnattached { clip } => write!(
                f,
                "arrow `{clip}` is attached to a clip that is not on screen while it is, \
                 so it was left out"
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
            Self::GeneratedMissing {
                clip,
                asset,
                stood_in,
            } => write!(
                f,
                "clip `{clip}` shows asset `{asset}`, which the project says was \
                 generated but has no file on disk — {stood_in} stood in for it"
            ),
            Self::UnknownIcon { clip, asset, named } => write!(
                f,
                "clip `{clip}` shows asset `{asset}`, which names no icon this build \
                 ships (`{named}`), so it drew nothing"
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
    /// How loud it came out — the mix, and each clip's contribution to it.
    /// `None` when there is no audio stream to measure.
    ///
    /// A **signal**, not a gate: nothing here can fail a render, because there
    /// is no correct loudness. What it replaces is a habit — running
    /// `ffmpeg -af volumedetect` by hand after every render — with a number the
    /// command says on its own.
    pub levels: Option<SoundLevels>,
    /// Everything the render wants a second look at. Empty is the good case;
    /// a caller that ignores this is the failure mode [`Note`] exists for.
    pub notes: Vec<Note>,
    /// What the file actually contains, stretch by stretch — the cut as the
    /// plan sequenced it, in seconds as well as frames.
    ///
    /// Carried on the report rather than left to the caller to re-derive,
    /// because the plan this came off is the one that was rendered: it has been
    /// probed, clamped to the range asked for, and had its media resolved.
    /// Building a second plan to describe the first is how the two drift.
    pub description: Description,
}

impl RenderReport {
    /// How long the output runs, in wall-clock seconds.
    pub fn seconds(&self) -> f64 {
        self.fps.seconds(Frames(self.frames))
    }
}
