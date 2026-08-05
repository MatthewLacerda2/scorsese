//! What can go wrong between a line in the document and an MP3 in
//! `generated/` — in two types, because there are two kinds of wrong.
//!
//! [`Incomplete`] is **one line's** problem: no words yet, no voice chosen yet,
//! more characters than the vendor speaks. [`SpeechError`] stops the **whole
//! run**: no key, the ceiling reached, a disk that will not take a file.
//!
//! Two types rather than one enum with a rule about which half is fatal,
//! because that rule would live in a comment and be enforced by nobody.
//! [`Brief::of`](super::Brief::of) can only return the first kind, so a run
//! **cannot** be stopped by a half-written line — that is a fact about the
//! signature rather than about the care taken at a call site.
//!
//! The reason it matters: a cut being written normally has a line in it that
//! nobody has chosen the voice for yet. Stopping the run there would mean
//! re-speaking nineteen paid-for lines in order to add the twentieth.

use std::path::PathBuf;

use scorsese_core::{AssetId, AssetKind};

use crate::credentials::{CredentialError, OverBudget};
use crate::prices::UnpricedSpeech;

use super::provider::ProviderError;

/// Why one line is not ready to be spoken.
///
/// Every variant is a thing somebody can finish writing. None of them says
/// anything about the other lines in the project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Incomplete {
    /// The asset asked for is not one that generates speech.
    #[error("{id} is a {kind:?} asset, and only generated_audio assets generate speech")]
    NotGeneratedAudio {
        /// The asset asked about.
        id: AssetId,
        /// What it actually is.
        kind: AssetKind,
    },

    /// A line with nothing in it.
    #[error("{id} has no prompt — there is nothing to speak")]
    NoPrompt {
        /// The asset with the empty brief.
        id: AssetId,
    },

    /// A line with nobody to speak it.
    ///
    /// Refused here rather than in validation, and the difference is the whole
    /// design: a narration whose voice has not been chosen is a legitimate
    /// document, exactly as a shot with no prompt yet is. There is no default
    /// to fall back on — every one of the vendor's Default voices expires on
    /// 2026-12-31 — so this is where it has to be caught, at the moment of
    /// spending rather than the moment of writing.
    #[error("{id} has no voice — choose one before generating this line")]
    NoVoice {
        /// The asset with no voice.
        id: AssetId,
    },

    /// More characters than the vendor speaks in one request.
    ///
    /// Validation reports this too, and this is not a duplicate: validation is
    /// a report somebody may not have run, and this is the last check before
    /// money moves. The cost of the second one is an `if`.
    #[error("{id} asks for {found} characters, and at most {max} are spoken in one request")]
    TooLong {
        /// The asset asking for it.
        id: AssetId,
        /// How many characters it holds.
        found: usize,
        /// How many the vendor takes.
        max: usize,
    },
}

/// Why a narration run stopped.
///
/// Every variant makes the **remaining** lines impossible too, which is the
/// test for belonging here rather than in [`Incomplete`].
///
/// Its own type rather than
/// [`video::GenerationError`](crate::video::GenerationError) shared, because a
/// shared one would carry `NoSuchStill` for a request that has no stills and
/// every reader would have to work out which half applied. What the two
/// genuinely have in common — [`CredentialError`], [`OverBudget`] — is shared.
#[derive(Debug, thiserror::Error)]
pub enum SpeechError {
    /// No key for the provider.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The run would cross the ceiling somebody set.
    #[error(transparent)]
    OverBudget(#[from] OverBudget),

    /// No published rate for the model asked for, so there is no price and the
    /// budget cannot be checked against one.
    #[error(transparent)]
    Unpriced(#[from] UnpricedSpeech),

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

    /// The provider could not be reached at all — as distinct from the provider
    /// answering and refusing, which is an outcome for one line.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}
