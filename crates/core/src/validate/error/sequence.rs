//! What an image sequence can get wrong.
//!
//! Split off for [`super::ImageProblem`]'s reason: the block is present and
//! each field in it is legal, and what is wrong is what the stills it names
//! turn out to be. Every finding is answerable from the document, so a
//! sequence that could never play is refused before a render starts on it.

use crate::asset::{AssetId, AssetKind};

/// One thing wrong with the stills an `image_sequence` plays.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SequenceProblem {
    /// A sequence of nothing. There is no picture to show at any instant.
    #[error("asset `{asset}` is an image sequence with no stills")]
    NoStills {
        /// The empty sequence.
        asset: AssetId,
    },

    /// A hold of zero frames: every still gone before it is shown.
    #[error("asset `{asset}` holds each still for 0 frames — a hold is at least 1")]
    NoHold {
        /// The sequence.
        asset: AssetId,
    },

    /// A still named by an id that is not in the assets table.
    #[error("asset `{asset}` plays `{still}` as a still, and no asset has that id")]
    UnknownStill {
        /// The sequence naming it.
        asset: AssetId,
        /// The id that resolves to nothing.
        still: AssetId,
    },

    /// A still that is not an imported picture.
    ///
    /// Only `image`: a generated still may not exist yet, and a sequence is
    /// a run of files someone already has. A sequence inside a sequence is
    /// refused by the same rule.
    #[error("asset `{asset}` plays `{still}` as a still, but that asset is a {kind:?}")]
    NotAnImage {
        /// The sequence naming it.
        asset: AssetId,
        /// The id that points at the wrong kind.
        still: AssetId,
        /// What that asset actually is.
        kind: AssetKind,
    },

    /// Stills in more than one file format.
    ///
    /// A sequence is decoded as one stream, and one stream has one decoder: a
    /// jpeg after a png is not a picture the png decoder can read. A rendered
    /// frame directory is one format by construction, so this is nearly always
    /// a stray file that was never meant to be a frame.
    #[error(
        "asset `{asset}` plays `{still}`, a .{found} file, among .{expected} stills — \
         every still of a sequence is one format"
    )]
    MixedFormats {
        /// The sequence.
        asset: AssetId,
        /// The first still in another format.
        still: AssetId,
        /// The format of the sequence's first still.
        expected: String,
        /// The format of this one.
        found: String,
    },

    /// Stills of more than one size, where both sizes have been measured.
    ///
    /// A clip fits **the sequence** to the frame, once, from its first still's
    /// size — so a still of another shape would be stretched to it rather than
    /// fitted. Refused rather than stretched, because a frame that jumps in
    /// shape partway through a timelapse is a mistake nobody wants made for
    /// them.
    #[error(
        "asset `{asset}` plays `{still}`, which is {found_width}x{found_height}, among \
         {width}x{height} stills — every still of a sequence is one size"
    )]
    MixedSizes {
        /// The sequence.
        asset: AssetId,
        /// The first still of another size.
        still: AssetId,
        /// The first still's width.
        width: u32,
        /// The first still's height.
        height: u32,
        /// This still's width.
        found_width: u32,
        /// This still's height.
        found_height: u32,
    },
}
