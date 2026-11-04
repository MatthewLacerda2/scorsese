//! What a generated sound effect's request can get wrong (#1265).
//!
//! The sibling of [`super::MusicProblem`], answerable from the document and
//! cheaper to find here than in a refused, billed round trip.

use crate::asset::AssetId;
use crate::validate::field::AssetField;

/// One thing wrong with what a generated sound effect is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SoundProblem {
    /// A `sound` block beside a `speech` or a `music` block on one asset.
    ///
    /// Which block an asset carries is what it is — a line, a track or a
    /// sound — so carrying two says two things and one would be ignored.
    #[error(
        "asset `{asset}` carries both a `sound` and a `{other}` block — \
         it is one thing, so keep the block it is"
    )]
    TwoBlocks {
        /// The asset carrying both.
        asset: AssetId,
        /// The other block it carries.
        other: AssetField,
    },

    /// A length the vendor will not make.
    #[error(
        "asset `{asset}` asks for {found} seconds of sound, and the vendor makes \
         between {min} and {max}"
    )]
    Length {
        /// The asset asking for it.
        asset: AssetId,
        /// The seconds it asks for.
        found: f64,
        /// The shortest the vendor takes.
        min: f64,
        /// The longest the vendor takes.
        max: f64,
    },
}
