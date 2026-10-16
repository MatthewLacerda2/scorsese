//! What can go wrong between a track in the document and an MP3 in
//! `generated/` — one track's problem ([`Incomplete`]), or the whole run's
//! ([`MusicError`]), split for [`speech`](crate::speech)'s reason: a brief can
//! only return the first, so a half-written track cannot stop a run.

use std::path::PathBuf;

use scorsese_core::AssetId;

use crate::credentials::{CredentialError, OverBudget};

use super::provider::ProviderError;

/// Why one track is not ready to be composed. Every variant is a thing
/// somebody can finish writing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Incomplete {
    /// The asset is not a track: not `generated_audio`, or one with no
    /// `music` block, which makes it a line.
    #[error("{id} is not a track — only a generated_audio asset with a `music` block is composed")]
    NotATrack {
        /// The asset asked about.
        id: AssetId,
    },

    /// A track with no description.
    #[error("{id} has no prompt — there is nothing to compose from")]
    NoPrompt {
        /// The asset with the empty brief.
        id: AssetId,
    },

    /// A track with no length.
    ///
    /// Refused here rather than in validation, as a line's missing voice is:
    /// a sketch nobody has timed yet is a legitimate document, and the price
    /// is per minute, so there is nothing to quote until it is set.
    #[error("{id} has no length — set `music.seconds` before generating this track")]
    NoLength {
        /// The asset with no length.
        id: AssetId,
    },

    /// A length the vendor will not compose. Validation reports it too; this
    /// is the last check before money moves.
    #[error("{id} asks for {found} seconds of music, and the vendor composes {min} to {max}")]
    Length {
        /// The asset asking for it.
        id: AssetId,
        /// The seconds it asks for.
        found: u32,
        /// The shortest the vendor takes.
        min: u32,
        /// The longest.
        max: u32,
    },
}

/// Why a run of music stopped. Every variant makes the remaining tracks
/// impossible too.
#[derive(Debug, thiserror::Error)]
pub enum MusicError {
    /// No key for the provider.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The run would cross the ceiling somebody set.
    #[error(transparent)]
    OverBudget(#[from] OverBudget),

    /// No asset by that id.
    #[error("no asset called {id} in this project")]
    NoSuchAsset {
        /// The id nothing answers to.
        id: AssetId,
    },

    /// The generated file could not be written into `generated/`.
    #[error("writing {}: {source}", path.display())]
    Write {
        /// Where it was going.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },

    /// The provider could not be reached at all.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}
