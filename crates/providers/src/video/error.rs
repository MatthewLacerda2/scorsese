//! What can go wrong between a brief in the document and a file in
//! `generated/`.

use std::path::PathBuf;

use scorsese_core::{AssetId, AssetKind};

use crate::credentials::{CredentialError, OverBudget};
use crate::prices::Unpriced;

use super::provider::ProviderError;

/// Why a generation did not happen.
///
/// A **refusal of the run**, and only that. What one asset did — cached,
/// queued, failed at the provider — is an [`Outcome`](super::Outcome) rather
/// than an error, because a run of twenty shots where one prompt is rejected
/// should leave nineteen shots generated and say so. An error here stops
/// everything, so the variants are the things that make the whole run
/// impossible.
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    /// No key for the provider.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The run would cross the ceiling somebody set.
    #[error(transparent)]
    OverBudget(#[from] OverBudget),

    /// The vendor does not sell what the brief asks for, so there is no price
    /// and the budget cannot be checked against one.
    #[error(transparent)]
    Unpriced(#[from] Unpriced),

    /// The asset asked for is not one that generates video.
    #[error("{id} is a {kind:?} asset, and only generated_video assets generate video")]
    NotAGeneratedVideo {
        /// The asset asked about.
        id: AssetId,
        /// What it actually is.
        kind: AssetKind,
    },

    /// No asset by that id.
    #[error("no asset called {id} in this project")]
    NoSuchAsset {
        /// The id nothing answers to.
        id: AssetId,
    },

    /// A brief with nothing in it. Refused before anything is spent, because
    /// what comes back from an empty prompt is a video nobody asked for and a
    /// charge nobody meant.
    #[error("{id} has no prompt — there is nothing to generate")]
    NoPrompt {
        /// The asset with the empty brief.
        id: AssetId,
    },

    /// A brief names a still that is not in the assets table.
    #[error("a brief names the image {id}, and no asset answers to it")]
    NoSuchStill {
        /// The id the brief named.
        id: AssetId,
    },

    /// A brief names a still whose asset carries no file.
    #[error("the image {id} has no file to send")]
    StillHasNoFile {
        /// The asset with no path.
        id: AssetId,
    },

    /// A still would not read off disk.
    #[error("reading the image {id} at {}: {source}", path.display())]
    ReadStill {
        /// Which still.
        id: AssetId,
        /// Where it should have been.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
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

    /// The provider could not be reached at all — as distinct from the provider
    /// answering and refusing, which is an outcome for one asset.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}
