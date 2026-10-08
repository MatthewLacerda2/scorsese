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
    /// change, and the sizes it does draw, so the fix is in the message.
    #[error(
        "asset `{asset}` asks for a {resolution} still, and the `{model}` model does not draw \
         that size — it draws {draws}"
    )]
    ResolutionUnsupported {
        /// The asset asking for it.
        asset: AssetId,
        /// The model that cannot.
        model: &'static str,
        /// The size as written.
        resolution: &'static str,
        /// The sizes it does draw, listed.
        draws: String,
    },

    /// A shape the chosen model does not draw — one of the long strips, on a
    /// model that draws only the ten common ratios.
    #[error(
        "asset `{asset}` asks for a {aspect} still, and the `{model}` model does not draw \
         that shape — use `nano_banana_2.1` or `flash`, or another aspect"
    )]
    AspectUnsupported {
        /// The asset asking for it.
        asset: AssetId,
        /// The model that cannot.
        model: &'static str,
        /// The ratio as written.
        aspect: &'static str,
    },

    /// More references of one kind than the chosen model accepts.
    ///
    /// Refused with the number, never truncated: which picture to drop is the
    /// editor's choice, not the sender's.
    #[error("asset `{asset}` names {found} {field}, and the `{model}` model takes at most {max}")]
    TooManyReferenceImages {
        /// The asset naming them.
        asset: AssetId,
        /// The model asked.
        model: &'static str,
        /// The field holding them: `reference_images`, `character_images` or
        /// `style_images`.
        field: &'static str,
        /// How many were written.
        found: usize,
        /// How many the model takes, zero when it takes none of that kind.
        max: usize,
    },

    /// A thinking level the chosen model does not offer.
    #[error(
        "asset `{asset}` asks the `{model}` model to think at `{thinking}`, and it offers {offers}"
    )]
    ThinkingUnsupported {
        /// The asset asking for it.
        asset: AssetId,
        /// The model that cannot.
        model: &'static str,
        /// The level as written.
        thinking: &'static str,
        /// The levels it does offer, listed, or that it offers none.
        offers: String,
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
