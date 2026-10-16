//! Why a voice was not cloned, or not forgotten.
//!
//! The rule the design errors next door keep holds here too: every variant
//! exists because its **advice differs**, and what a listing already names —
//! no key, a key without a scope, a vendor that would not answer — arrives as
//! [`VoiceError`] and is carried through whole.

use scorsese_core::AssetId;

use super::super::VoiceError;
use super::super::design::DesignError;
use super::CONSENT;

/// Why a clone did not happen.
#[derive(Debug, thiserror::Error)]
pub enum CloneError {
    /// Something a listing already names.
    #[error(transparent)]
    Voice(#[from] VoiceError),

    /// The studio could not forget the voice — the same call a designed
    /// voice is forgotten with, so the same error.
    #[error(transparent)]
    Forget(#[from] DesignError),

    /// Consent was not given. Asked before anything else, every time: the
    /// vendor's API takes no such field, so this is where it is asked.
    #[error(
        "nothing was sent: cloning needs consent given for this voice. {CONSENT} Ask whoever \
         is speaking in the recordings, and clone only once they have said yes."
    )]
    NoConsent,

    /// No name for the voice.
    #[error(
        "a cloned voice needs a name: it is what the voice is called in the ElevenLabs account, \
         and the only thing it is recognisable by there. Nothing was sent."
    )]
    NoName,

    /// No recording to clone from.
    #[error(
        "a voice is cloned from recordings of it: name at least one audio asset — about a \
         minute or two of clear speech in all. Nothing was sent."
    )]
    NoRecordings,

    /// A recording that is not in the project.
    #[error("there is no asset {asset} in this project, so nothing was sent")]
    NoSuchAsset {
        /// The id that was asked for.
        asset: AssetId,
    },

    /// An asset that is not an imported recording.
    ///
    /// Only imported audio: a clone is of somebody's own recorded speech,
    /// never of a line a voice already narrated or of a video's soundtrack.
    #[error(
        "{asset} is {kind}, not an imported audio recording, so nothing was sent. Import the \
         recording as audio and clone from that."
    )]
    NotARecording {
        /// The asset.
        asset: AssetId,
        /// What it is instead.
        kind: String,
    },

    /// A recording whose file could not be read.
    #[error("could not read the recording {path}: {source}. Nothing was sent.")]
    Unreadable {
        /// Where it should have been.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },

    /// Instant Voice Cloning is not on this account's plan.
    #[error(
        "ElevenLabs would not clone a voice for this account: {said}\nInstant Voice Cloning \
         needs the Starter plan or above. Nothing was created."
    )]
    NotOnThisPlan {
        /// What the vendor said, verbatim.
        said: String,
    },

    /// A voice to forget that this project did not clone, or forgot already.
    #[error(
        "{voice_id} is not a voice this project cloned and still keeps, so nothing was \
         deleted. List the voices it cloned, and forget one by the id the list shows."
    )]
    NotKept {
        /// The id that was asked for.
        voice_id: String,
    },

    /// The record of a cloned voice could not be written.
    ///
    /// Fatal: the voice exists at the vendor and takes a slot, and a record
    /// that quietly did not save is a slot nobody can account for.
    #[error("{doing}: {source}")]
    Write {
        /// What was being written when it failed.
        doing: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}
