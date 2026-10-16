//! What a generated track's request can get wrong (#1264).
//!
//! The sibling of [`super::SpeechProblem`]. Both are answerable from the
//! document, and both cost nothing to find here where the vendor would charge
//! a round trip to say less.

use crate::asset::AssetId;

/// One thing wrong with what a generated track is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MusicProblem {
    /// A `speech` block and a `music` block on one asset.
    ///
    /// Which block an asset carries is what it is — a line or a track — so
    /// carrying both says two things and one of them would be ignored.
    #[error(
        "asset `{asset}` carries both a `speech` and a `music` block — \
         it is a line or a track, so keep the one it is"
    )]
    BothBlocks {
        /// The asset carrying both.
        asset: AssetId,
    },

    /// A length the vendor will not compose.
    #[error(
        "asset `{asset}` asks for {found} seconds of music, and the vendor composes \
         between {min} and {max} — split a longer track across two assets"
    )]
    Length {
        /// The asset asking for it.
        asset: AssetId,
        /// The seconds it asks for.
        found: u32,
        /// The shortest the vendor takes.
        min: u32,
        /// The longest the vendor takes.
        max: u32,
    },
}
