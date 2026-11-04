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
//! **Or one page for the whole track** ([`Look::Page`], #1550): the
//! captions' words handed to a page the caller writes, which lights each word
//! as it is said from `scorsese.words`. The page is an ordinary `html` asset
//! under `pages/`, `caption-page-<track>`, played by one clip of that name
//! from the first caption's arrival to the last one's leaving.
//!
//! **What it wrote, it recognises by name.** A caption's asset and clip are
//! both called `caption-<narration clip>-<n>`, and a run first takes away
//! every text clip so called and the caption assets left unused — then writes
//! the captions afresh. So a re-run after a line is regenerated or moved
//! re-times its captions, and nothing has to be detected; a hand-made clip,
//! called anything else, is never touched. Editing a caption's words by hand
//! is an edit the next run replaces, which its asset's note says.
//!
//! **Transcribed footage is captioned like a line** (#1549): an imported
//! video or sound whose speech was transcribed has the same timings beside
//! it, on whatever track its clip is — unless its clip is held silent
//! throughout, the picture kept under another voice.
//!
//! **A line with no timings is named and skipped, never guessed** — speech
//! nobody transcribed on a track the caller named, or a narration generated
//! before timings were kept.
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

/// The page a caption track is played by: written from the run and its
/// captions, in the order they arrive — each with its words named as
/// `scorsese.words` names them under `<line clip id>/`.
pub type PageHtml = fn(&Captioning, &[Chunk]) -> String;

/// What a run writes the captions as.
#[derive(Debug, Clone, Copy)]
pub enum Look {
    /// One `text` asset and clip per caption, in [`Captioning::style`].
    Text,
    /// One page for the whole track, written by the function given.
    Page(PageHtml),
}

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
    /// The tracks whose clips are captioned — those that can carry speech: a
    /// generated line, a video, a sound. Empty means generated narration on
    /// any audio track and transcribed recordings on any track not silenced
    /// throughout ([`Captioning::volume`]) — so music, or footage nobody
    /// transcribed, or a generated track, is not reported as a line without
    /// timings.
    pub narration: Vec<TrackId>,
    /// How lines are cut and timed.
    pub chunking: Chunking,
    /// Text clips, or one page.
    pub look: Look,
    /// What every caption is set in. Its `reveal` block is replaced: a caption
    /// arrives whole, its words rising together. A page reads it as it likes.
    pub style: TextStyle,
    /// How far up from the frame's bottom edge the captions sit, as a fraction
    /// of its height.
    pub lift: f64,
    /// How many frames a text caption takes to arrive.
    pub arrive: u64,
    /// The property that brings a text in a piece at a time.
    pub reveal: PropertyPath,
    /// The property that offsets a layer vertically.
    pub height: PropertyPath,
    /// The property that sets how loud a clip plays, as a multiplier: with no
    /// track named, a clip held at `0` on it throughout says nothing, and is
    /// not captioned. `None` hears every clip.
    pub volume: Option<PropertyPath>,
}

#[cfg(test)]
mod tests;
