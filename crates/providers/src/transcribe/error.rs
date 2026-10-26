//! Why a transcription did not happen.

use scorsese_core::{AssetId, AssetKind, ProjectPath};

use super::START_IT;

/// Why a recording was not transcribed.
#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    /// Nothing answers at the server's address.
    #[error("the Whisper server is not answering at {address} ({why}): {START_IT}")]
    NotRunning {
        /// Where it was looked for.
        address: String,
        /// What the transport said: refused, or a wait that ran out.
        why: String,
    },
    /// It answered with a refusal, carried whole.
    #[error(
        "the Whisper server refused ({status}): {body} — a language it does not know answers \
         500; force one only with a code like `pt` or `en`"
    )]
    Refused {
        /// The status code.
        status: u16,
        /// What it said.
        body: String,
    },
    /// It answered with something that is not a transcription.
    #[error("the Whisper server answered with something unreadable: {0}")]
    Unreadable(String),
    /// A language that is not a code.
    #[error(
        "`{0}` is not a language code: give one like `pt` or `en`, or leave it out to have it detected"
    )]
    Language(String),
    /// No asset is so called.
    #[error("there is no asset `{0}`")]
    NoSuchAsset(AssetId),
    /// A generated line, which has its timings from the provider that spoke it.
    #[error("`{0}` is a generated line, which has word timings of its own from when it was spoken")]
    Generated(AssetId),
    /// An asset that is not a recording.
    #[error("`{asset}` is {kind}: only an imported video or sound can be transcribed")]
    NotARecording {
        /// Which.
        asset: AssetId,
        /// What it is.
        kind: AssetKind,
    },
    /// A correction to a recording that has no transcript.
    #[error("`{0}` has no transcript to correct: transcribe it first")]
    Untranscribed(AssetId),
    /// A correction that names no word, or one word twice.
    #[error(transparent)]
    Fix(#[from] scorsese_core::words::FixError),
    /// The recording's file is not there.
    #[error("`{asset}`'s file {path} is not in the project")]
    Missing {
        /// Which.
        asset: AssetId,
        /// Where it should be.
        path: ProjectPath,
    },
    /// Its speech could not be decoded.
    #[error("`{asset}`'s sound could not be read: {why}")]
    Decode {
        /// Which.
        asset: AssetId,
        /// What the decoder said.
        why: String,
    },
    /// The words could not be written beside it.
    #[error("the transcript {path} could not be written: {source}")]
    Write {
        /// Where.
        path: ProjectPath,
        /// What the filesystem said.
        source: std::io::Error,
    },
}
