//! What the assets-table checks can find: identity, paths, and the fields a
//! kind requires.

use crate::asset::{AssetId, AssetKind};
use crate::path::{PathProblem, ProjectPath};
use crate::validate::error::{IconProblem, ShapeProblem, SpeechProblem, VideoProblem};
use crate::validate::field::AssetField;

/// One thing wrong with a row of the assets table.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AssetProblem {
    /// Ids are how clips name assets, so a repeat makes every reference to it
    /// ambiguous.
    #[error("asset id `{id}` is used more than once")]
    DuplicateAssetId {
        /// The id claimed twice.
        id: AssetId,
    },

    /// A path that would not survive `scp -r` — absolute, backslashed, or
    /// climbing out of the project root.
    #[error("asset `{asset}`: path `{path}` {problem}")]
    BadPath {
        /// The asset carrying it.
        asset: AssetId,
        /// The path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// A field the asset's kind requires, absent.
    ///
    /// One variant rather than one per field: which fields a kind needs is
    /// [`AssetField`]'s to explain, and saying it again per error is how the
    /// same reasoning ends up written twice and maintained once.
    #[error("asset `{asset}` is a {kind:?} and needs a `{field}`")]
    MissingField {
        /// The asset that is short a field.
        asset: AssetId,
        /// Which field, and why that kind wants it.
        field: AssetField,
        /// The kind that requires it.
        kind: AssetKind,
    },

    /// A field belonging to some other kind.
    ///
    /// The mirror of [`AssetProblem::MissingField`], and worth reporting for
    /// the same reason: nothing would ever read it, and silence about it would
    /// look exactly like having read it. Usually a `kind` that was meant to be
    /// something else.
    #[error("asset `{asset}` is a {kind:?}, so `{field}` does not apply to it")]
    StrayField {
        /// The asset carrying it.
        asset: AssetId,
        /// Which field, and where it does belong.
        field: AssetField,
        /// The kind that has no use for it.
        kind: AssetKind,
    },

    /// A text asset carries its content inline; without it there is nothing
    /// to draw.
    #[error("asset `{asset}` is a text asset and needs `text` content")]
    MissingText {
        /// The empty text asset.
        asset: AssetId,
    },

    /// A font file that would not survive `scp -r`, under the same rules every
    /// other path in the document obeys. Named apart from
    /// [`AssetProblem::BadPath`] because a text asset has two paths in play —
    /// its font and nothing else — and "which path" is the first thing a reader
    /// needs.
    #[error("asset `{asset}`: font `{path}` {problem}")]
    BadFontPath {
        /// The text asset naming it.
        asset: AssetId,
        /// The font path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// A number that is not a weight on anybody's scale.
    ///
    /// The bounds are OpenType's for the `wght` axis, so this is checkable from
    /// the document alone. Whether a *particular* face reaches the weight asked
    /// for is narrower and only the file can answer it, so that refusal waits
    /// until the render opens it.
    #[error("asset `{asset}`: weight {weight} is outside the {min}–{max} a font weight can be")]
    WeightOutOfRange {
        /// The text asset naming it.
        asset: AssetId,
        /// The number as written.
        weight: u16,
        /// The lightest weight the format allows.
        min: u16,
        /// The heaviest weight the format allows.
        max: u16,
    },

    /// A recipe path that would not survive `scp -r`, under the same rules
    /// every other path in the document obeys. Named apart from
    /// [`AssetProblem::BadPath`] because a synthesis asset has two paths in
    /// play — its recipe and its baked media — and "which path" is the first
    /// thing a reader needs.
    #[error("asset `{asset}`: recipe `{path}` {problem}")]
    BadRecipePath {
        /// The synthesis asset naming it.
        asset: AssetId,
        /// The recipe path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// `generated` claims the media exists, so something has to say where.
    /// Usually a state edited by hand ahead of the generation.
    #[error("asset `{asset}` is in state `generated` but has no `path` to the generated file")]
    GeneratedWithoutPath {
        /// The asset claiming media it cannot produce.
        asset: AssetId,
    },

    /// Something wrong with what a generated video is asking for.
    ///
    /// Its own catalogue rather than more variants here, because these are
    /// findings about a *combination* of fields rather than about one of them
    /// — see [`VideoProblem`] for why that difference is worth a split.
    #[error(transparent)]
    Video(#[from] VideoProblem),

    /// Something wrong with the outline a `shape` asset describes.
    ///
    /// Split for [`VideoProblem`]'s reason — findings about the numbers inside
    /// one block rather than about whether the block is there. See
    /// [`ShapeProblem`].
    #[error(transparent)]
    Shape(#[from] ShapeProblem),

    /// Something wrong with the symbol an `icon` asset describes.
    ///
    /// Split for [`ShapeProblem`]'s reason, and kept apart from it for the
    /// reason the kinds are kept apart: the same field names mean different
    /// things on the two, so one catalogue would report a thickness against the
    /// wrong unit. See [`IconProblem`].
    #[error(transparent)]
    Icon(#[from] IconProblem),

    /// Something wrong with what a spoken line is asking for.
    ///
    /// Split from the rest for the reason [`VideoProblem`] is, and one more of
    /// its own: one finding in it describes a request the vendor **accepts**
    /// and charges for, so the document is the only place it can be caught.
    /// See [`SpeechProblem`].
    #[error(transparent)]
    Speech(#[from] SpeechProblem),

    /// Not the shape a SHA-256 comes in, so it can never match a real file —
    /// truncated, uppercase, or an algorithm that is not SHA-256.
    #[error("asset `{asset}`: sha256 `{value}` is not 64 lowercase hex characters")]
    BadSha256 {
        /// The asset carrying it.
        asset: AssetId,
        /// The string that is not a hash.
        value: String,
    },

    /// A rim colour on a text style with no thickness to draw it at.
    ///
    /// The same refusal a shape's border gets, for the same reason: *I meant no
    /// edge* and *I meant an edge and got the width wrong* look identical in
    /// the rendered frame, and only one of them is what the document says. On a
    /// caption it is the worse of the two to guess at, since the whole job of
    /// the rim is to be there over footage nobody has looked at yet.
    #[error("asset `{asset}`: a text stroke is {width} thick, so nothing would be drawn")]
    StrokeWithoutWidth {
        /// The text asset.
        asset: AssetId,
        /// The thickness as written.
        width: f64,
    },
}
