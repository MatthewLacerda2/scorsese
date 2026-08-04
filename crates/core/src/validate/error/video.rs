//! What a generated video's request can get wrong.
//!
//! Its own catalogue, folded into [`super::AssetProblem`], because these are
//! not the usual missing-or-stray finding: the fields are all present and all
//! individually legal, and what is wrong is the *combination*. The provider
//! couples choices that look independent — a raster fixes a length, a cheaper
//! tier withdraws a capability — so every problem here has to name the other
//! choice as well as the broken one, or the reader is told what they cannot
//! have without being told which of their decisions took it away.
//!
//! Every one of these is checkable from the document alone, which is the whole
//! reason to check it: a request rejected by the provider costs a round trip
//! and, on a bad day, an argument about whether the money was spent.

use crate::asset::{AssetId, AssetKind, ClipSeconds, LengthLock, VideoModel};

/// One thing wrong with what a generated video is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VideoProblem {
    /// A length the provider will not generate, because another choice fixed
    /// it at eight seconds.
    ///
    /// The message names the choice rather than only the rule, because that is
    /// the one the reader might trade away: dropping to 720p costs less than
    /// giving up the stills a shot is built from.
    #[error(
        "asset `{asset}` asks for {asked} seconds, but {cause} is only generated at 8 — \
         change one or the other"
    )]
    LengthLocked {
        /// The asset asking for it.
        asset: AssetId,
        /// The length as written.
        asked: u32,
        /// The choice that fixed it.
        cause: &'static str,
    },

    /// Reference images on a tier that has none.
    ///
    /// Worth its own refusal rather than a silently dropped field: reference
    /// images are what keep a subject looking like itself from shot to shot, so
    /// quietly ignoring them is how a character changes face halfway through a
    /// cut with nothing having reported a problem.
    #[error(
        "asset `{asset}` names reference images, but the `{model}` tier does not take them — \
         use `fast`, or drop the images"
    )]
    ReferenceImagesUnsupported {
        /// The asset naming them.
        asset: AssetId,
        /// The tier that cannot take them.
        model: &'static str,
    },

    /// More reference images than the provider accepts.
    #[error("asset `{asset}` names {found} reference images, and at most {max} are accepted")]
    TooManyReferenceImages {
        /// The asset naming them.
        asset: AssetId,
        /// How many were written.
        found: usize,
        /// How many the provider takes.
        max: usize,
    },

    /// A still to end on, with nothing to start from.
    ///
    /// A last image asks for the journey between two pictures, so on its own it
    /// is not a smaller version of that request — it is a request with no
    /// meaning, and most often a `first_image` that was meant to be filled in.
    #[error(
        "asset `{asset}` has a `last_image` and no `first_image`: \
         a last still is the end of a journey between two, so it needs the one it starts from"
    )]
    LastImageWithoutFirst {
        /// The asset naming it.
        asset: AssetId,
    },

    /// A still named by an id that is not in the assets table.
    #[error("asset `{asset}` names `{referenced}` as a still, and no asset has that id")]
    UnknownImage {
        /// The asset naming it.
        asset: AssetId,
        /// The id that resolves to nothing.
        referenced: AssetId,
    },

    /// A still that is not a still.
    ///
    /// Every image handed to the provider is a picture, so an id pointing at a
    /// sound or a sketch is a mistake the document can catch — and catching it
    /// here is the difference between a validation message and a rejected
    /// request nobody expected.
    #[error("asset `{asset}` names `{referenced}` as a still, but that asset is a {kind:?}")]
    NotAnImage {
        /// The asset naming it.
        asset: AssetId,
        /// The id that points at the wrong kind.
        referenced: AssetId,
        /// What that asset actually is.
        kind: AssetKind,
    },
}

impl VideoProblem {
    /// The refusal for a length another choice has fixed.
    pub(crate) fn locked(asset: &AssetId, asked: ClipSeconds, lock: LengthLock) -> Self {
        Self::LengthLocked {
            asset: asset.clone(),
            asked: asked.get(),
            cause: lock.cause(),
        }
    }

    /// The refusal for reference images on a tier without them.
    pub(crate) fn unsupported(asset: &AssetId, model: VideoModel) -> Self {
        Self::ReferenceImagesUnsupported {
            asset: asset.clone(),
            model: model.as_str(),
        }
    }
}
