//! What a generated still's request can get wrong.
//!
//! The sibling of [`super::VideoProblem`], for the same reason it is split
//! off: the fields are present and individually legal, and what is wrong is a
//! combination the vendor will not draw or a picture it cannot be shown. Every
//! one is answerable from the document, so it is refused here — before a
//! request is sent, rather than by a round trip after it.

use crate::asset::{AssetId, AssetKind};

/// One thing wrong with what a generated still is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ImageProblem {
    /// A size the chosen model does not draw.
    ///
    /// Names the model as well as the size, because either is the thing to
    /// change: the cheaper model draws 1K only, and a still that has to be 4K
    /// is a still that has to be drawn by the full one.
    #[error(
        "asset `{asset}` asks for a {resolution} still, and the `{model}` model does not draw \
         that size — use `flash`, or 1K"
    )]
    ResolutionUnsupported {
        /// The asset asking for it.
        asset: AssetId,
        /// The model that cannot.
        model: &'static str,
        /// The size as written.
        resolution: &'static str,
    },

    /// More reference images than the vendor accepts.
    #[error("asset `{asset}` names {found} reference images, and at most {max} are accepted")]
    TooManyReferenceImages {
        /// The asset naming them.
        asset: AssetId,
        /// How many were written.
        found: usize,
        /// How many the vendor takes.
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
        "asset `{asset}` names `{referenced}` as a reference image, but that asset is a {kind:?}"
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
