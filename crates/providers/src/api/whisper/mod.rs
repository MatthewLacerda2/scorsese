//! The local Whisper server's transcription endpoint (#1549).
//!
//! speaches (`deploy/whisper/Dockerfile`), an OpenAI-style server over
//! faster-whisper, run by the user or by the web app's stack — never a
//! vendor's: nothing here takes a key, and the server reaches no network.
//! Its `POST /v1/audio/transcriptions` takes OpenAI's form, and what is
//! particular to it, read in speaches 0.8.3's own `routers/stt.py` and checked
//! against its replies on 2026-10-26:
//!
//! - **Word timings take two fields**: `response_format=verbose_json` and
//!   `timestamp_granularities[]=word`. With either missing the reply has no
//!   `words`.
//! - **Each word arrives with a leading space** (`" Hello"`) and its
//!   punctuation (`" there,"`); times are seconds of the file sent.
//! - **`language` is optional**: without it the language is detected and
//!   named in the reply. One Whisper does not know answers `500 Internal
//!   Server Error`, in plain text.
//! - **A model is named by its hub id**, and one not baked into the image is
//!   a `404` with `{"detail": …}`.
//! - Silence is no error: `{"text": "", "words": []}`.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde::Deserialize;

use crate::api::http::{Caller, Form, HttpError};

/// The path the server transcribes at, under its address.
const PATH: &str = "/v1/audio/transcriptions";

/// The transcription endpoint of the server at one address.
#[derive(Debug, Clone)]
pub struct Transcriptions {
    url: String,
    caller: Caller,
}

/// One request: a WAV of the speech, the model, and a language if one is
/// forced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<'a> {
    /// The recording, as a WAV file's bytes.
    pub wav: Vec<u8>,
    /// The model's hub id, as the image was built with.
    pub model: &'a str,
    /// An ISO 639-1 code, or `None` to have it detected.
    pub language: Option<&'a str>,
}

impl Request<'_> {
    /// The form the server reads, the recording moved into it rather than
    /// copied.
    pub fn form(self) -> Form {
        let mut form = Form::new()
            .file("file", "speech.wav", "audio/wav", self.wav)
            .text("model", self.model)
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "word");
        if let Some(language) = self.language {
            form = form.text("language", language);
        }
        form
    }
}

/// The reply, as much of it as scorsese reads.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Verbose {
    /// The language heard, or the one forced.
    pub language: String,
    /// How long the recording is, in seconds.
    pub duration: f64,
    /// Every word, in order.
    #[serde(default)]
    pub words: Vec<Word>,
}

/// One word as the server sends it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Word {
    /// The word with its leading space and punctuation.
    pub word: String,
    /// When it begins, in seconds of the file.
    pub start: f64,
    /// When it ends.
    pub end: f64,
}

impl Transcriptions {
    /// The endpoint of the server at `address` (`http://host:port`).
    pub fn new(address: &str) -> Self {
        Self {
            url: format!("{}{PATH}", address.trim_end_matches('/')),
            caller: Caller::anonymous(),
        }
    }

    /// Where requests go.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Sends `request` and waits for its words, for as long as `within`; the
    /// connection closed once `stopped` is set, sent or not.
    pub fn transcribe(
        &self,
        request: Request<'_>,
        within: Duration,
        stopped: &Arc<AtomicBool>,
    ) -> Result<Verbose, HttpError> {
        self.caller
            .post_multipart_until(&self.url, &request.form(), within, stopped)
    }
}

#[cfg(test)]
mod tests;
