//! What a generated still's request can get wrong.
//!
//! The sibling of [`super::VideoProblem`], for the same reason it is split
//! off: the fields are present and individually legal, and what is wrong is
//! more references than the vendor takes or a picture it cannot be shown.
//! Every one is answerable from the document, so it is refused here — before a
//! request is sent, rather than by a round trip after it.

use crate::asset::{AssetId, AssetKind};

/// One thing wrong with what a generated still is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ImageProblem {
    /// More references of one kind than the model accepts.
    ///
    /// Refused with the number, never truncated: which picture to drop is the
    /// editor's choice, not the sender's.
    #[error("asset `{asset}` names {found} {field}, and a still takes at most {max}")]
    TooManyReferenceImages {
        /// The asset naming them.
        asset: AssetId,
        /// The field holding them: `reference_images` or `character_images`.
        field: &'static str,
        /// How many were written.
        found: usize,
        /// How many a still takes.
        max: usize,
    },

    /// A reference named by an id that is not in the assets table.
    #[error("asset `{asset}` names `{referenced}` as a reference image, and no asset has that id")]
    UnknownImage {
        /// The asset naming it.
        asset: AssetId,
        /// The id that resolves to nothing.
        referenced: AssetId,
    },

    /// A reference that is not a picture.
    #[error(
        "asset `{asset}` names `{referenced}` as a reference image, but that asset is of kind `{kind}`"
    )]
    NotAnImage {
        /// The asset naming it.
        asset: AssetId,
        /// The id that points at the wrong kind.
        referenced: AssetId,
        /// What that asset actually is.
        kind: AssetKind,
    },

    /// A still drawn from itself.
    ///
    /// It could never be generated: its own picture does not exist until it
    /// has been, so the brief would wait on itself for ever.
    #[error("asset `{asset}` names itself as a reference image")]
    ReferencesItself {
        /// The asset.
        asset: AssetId,
    },
}
