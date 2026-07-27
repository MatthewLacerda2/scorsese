//! Why a timeline cannot be sequenced into a render.
//!
//! Everything here is a refusal decided from the **document alone** — no file
//! is opened to reach one of these. What the disk has to say about a project
//! arrives later, as a [`crate::error::RenderError`] or, where a card can stand
//! in for what is missing, as a [`crate::report::Note`].

use scorsese_core::Frames;

use super::FrameRange;

/// Why a timeline cannot be sequenced into a render.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    /// Picture decides how long a render is, so a project with only audio has
    /// no length to give one.
    #[error("there is nothing to render: no video track has any clips")]
    NothingToRender,

    /// Typically a stale `--range` left over from a longer cut.
    #[error("range {range} selects no frames of a timeline {timeline_end} long")]
    EmptyRange {
        /// The range as asked for, after clamping.
        range: FrameRange,
        /// Where the timeline actually ends.
        timeline_end: Frames,
    },

    /// The project is internally inconsistent: validation should have caught
    /// this before a render was ever attempted.
    #[error("clip `{clip}` references asset `{asset}`, which is not in the assets table")]
    UnknownAsset {
        /// The clip holding the dangling reference.
        clip: String,
        /// The asset id nothing in the table matches.
        asset: String,
    },

    /// An imported asset with no path — a project edited by hand, or a
    /// migration that lost one. A *prompt* with no file is not this: it has a
    /// slug card to show instead, which is the whole of the sketch lifecycle.
    #[error("clip `{clip}` shows asset `{asset}`, which has no media file")]
    NoMedia {
        /// The clip with nothing to show.
        clip: String,
        /// The asset whose path is missing.
        asset: String,
    },
}
