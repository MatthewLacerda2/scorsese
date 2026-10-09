//! The narration's own words as on-screen captions (#965).
//!
//! Short-form video is watched muted most of the time, and what the narration
//! says is usually the best text the video has. So this turns each generated
//! line's **word timings** ([`crate::words`]) into captions: the line cut into
//! short pieces ([`chunks`]), each arriving on its first spoken word and
//! leaving when the next arrives ([`time`]).
//!
//! **What comes out is ordinary document content**, the bargain [`crate::dip`]
//! made for ducking: one `text` asset and one clip per caption, on a video
//! track the caller names, animated by ordinary keyframe tracks. There is no
//! caption kind and no caption stage in the compositor, and a person can edit
//! or delete any caption afterwards like anything else.
//!
//! **What it wrote, it recognises by name.** A caption's asset and clip are
//! both called `caption-<narration clip>-<n>`, and a run first takes away
//! every text clip so called and the caption assets left unused — then writes
//! the captions afresh. So a re-run after a line is regenerated or moved
//! re-times its captions, and nothing has to be detected; a hand-made clip,
//! called anything else, is never touched. Editing a caption's words by hand
//! is an edit the next run replaces, which its asset's note says.
//!
//! **A line with no timings is named and skipped, never guessed** — imported
//! speech, or a narration generated before timings were kept.
//!
//! Like [`crate::dip`], this names no property: the caller hands in the
//! property paths for the arrival and the height, and the style the text is
//! set in, since which words are real fonts and properties is the
//! compositor's to say.

mod chunk;
mod place;

use crate::keyframe::PropertyPath;
use crate::text::TextStyle;
use crate::timeline::TrackId;

pub use chunk::{Chunk, Chunking, chunks, time};
pub use place::{CaptionError, Captioned, caption};

/// Who signs the keyframe tracks a caption carries.
pub const TOOL: &str = "caption";

/// What a caption's asset and clip ids begin with — how a run knows its own
/// work.
pub const PREFIX: &str = "caption-";

/// Everything one run of captioning is asked.
#[derive(Debug, Clone)]
pub struct Captioning {
    /// The video track the captions go on. Made, on top of every other, when
    /// the project has no track so called.
    pub track: TrackId,
    /// The audio tracks whose lines are captioned. Empty means every audio
    /// track, where only generated narration counts — so music on its own
    /// track is not reported as a line without timings.
    pub narration: Vec<TrackId>,
    /// How lines are cut and timed.
    pub chunking: Chunking,
    /// What every caption is set in. Its `reveal` block is replaced: a caption
    /// arrives whole, its words rising together.
    pub style: TextStyle,
    /// How far up from the frame's bottom edge the captions sit, as a fraction
    /// of its height.
    pub lift: f64,
    /// How many frames a caption takes to arrive.
    pub arrive: u64,
    /// The property that brings a text in a piece at a time.
    pub reveal: PropertyPath,
    /// The property that offsets a layer vertically.
    pub height: PropertyPath,
}

#[cfg(test)]
mod tests;
