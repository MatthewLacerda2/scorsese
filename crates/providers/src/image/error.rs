//! What can go wrong between a still in the document and a JPEG in
//! `generated/` — in two types, for [`speech`](crate::speech)'s reason.
//!
//! [`Incomplete`] is **one still's** problem: no prompt yet, a reference not
//! generated yet. [`ImageError`] stops the **whole run**: no key, the ceiling
//! reached, a disk that will not take a file. [`Brief::of`](super::Brief::of)
//! can only return the first, so a half-written still cannot stop the others.

use std::path::PathBuf;

use scorsese_core::{AssetId, AssetKind};

use crate::credentials::{CredentialError, OverBudget};
use crate::prices::UnpricedImage;

use super::provider::ProviderError;

/// Why one still is not ready to be drawn.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Incomplete {
    /// The asset asked for is not one that generates a still.
    #[error("{id} is a {kind:?} asset, and only generated_image assets generate a still")]
    NotGeneratedImage {
        /// The asset asked about.
        id: AssetId,
        /// What it actually is.
        kind: AssetKind,
    },

    /// A brief with nothing in it.
    #[error("{id} has no prompt — there is nothing to draw")]
    NoPrompt {
        /// The asset with the empty brief.
        id: AssetId,
    },

    /// A reference picture that cannot be sent yet — missing, unreadable, or
    /// a generated still nobody has generated.
    #[error("{id}: {why}")]
    Reference {
        /// The still being drawn.
        id: AssetId,
        /// What is wrong with the reference, in the words reading it gave.
        why: String,
    },
}

/// Why a run of stills stopped. Every variant makes the **remaining** stills
/// impossible too.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// No key for the provider.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The run would cross the ceiling somebody set.
    #[error(transparent)]
    OverBudget(#[from] OverBudget),

    /// No published rate for what the brief asks, so the budget cannot be
    /// checked against one.
    #[error(transparent)]
    Unpriced(#[from] UnpricedImage),

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
    ///
    /// Unused by the run today — a sync provider's failure is an outcome for
    /// the one still — and kept so a caller resolving a key can report through
    /// the same type.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}
