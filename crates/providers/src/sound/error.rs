//! What can go wrong between a sound in the document and an MP3 in
//! `generated/` — one sound's problem ([`Incomplete`]), or the whole run's
//! ([`SoundError`]), split for [`music`](crate::music)'s reason: a brief can
//! only return the first, so a half-written sound cannot stop a run.

use std::path::PathBuf;

use scorsese_core::AssetId;

use crate::credentials::{CredentialError, OverBudget};

use super::provider::ProviderError;

/// Why one sound is not ready to be made. Every variant is a thing somebody
/// can finish writing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Incomplete {
    /// The asset is not a sound effect: not `generated_audio`, or one with no
    /// `sound` block.
    #[error(
        "{id} is not a sound effect — only a generated_audio asset with a `sound` block is made"
    )]
    NotASound {
        /// The asset asked about.
        id: AssetId,
    },

    /// A sound with no description.
    #[error("{id} has no prompt — there is nothing to make it from")]
    NoPrompt {
        /// The asset with the empty brief.
        id: AssetId,
    },

    /// A sound with no length.
    ///
    /// Refused here rather than in validation, as a track's is: a sketch
    /// nobody has timed yet is a legitimate document, and the price is per
    /// minute, so there is nothing to quote until it is set.
    #[error("{id} has no length — set `sound.seconds` before generating this sound")]
    NoLength {
        /// The asset with no length.
        id: AssetId,
    },

    /// A length the vendor will not make. Validation reports it too; this is
    /// the last check before money moves.
    #[error("{id} asks for {found} seconds of sound, and the vendor makes {min} to {max}")]
    Length {
        /// The asset asking for it.
        id: AssetId,
        /// The seconds it asks for.
        found: f64,
        /// The shortest the vendor takes.
        min: f64,
        /// The longest.
        max: f64,
    },
}

/// Why a run of sound effects stopped. Every variant makes the remaining
/// sounds impossible too.
#[derive(Debug, thiserror::Error)]
pub enum SoundError {
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
