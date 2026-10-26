//! The server behind a trait.

use std::str::FromStr;
use std::time::Duration;

use scorsese_core::words::{Word, Words};

use crate::api::http::HttpError;
use crate::api::whisper::{Request, Transcriptions};

use super::{MODEL, TranscribeError};

/// How long a request may take before its speech: the model loading, the
/// first time after the server starts.
const SETTLING: Duration = Duration::from_secs(5 * 60);

/// How many seconds of waiting each second of speech is allowed. Measured at
/// about two on a desktop CPU (#1549); four for a slower machine, or one
/// busy with a render.
const PER_SECOND: f64 = 4.0;

/// Bytes in a second of what is sent: sixteen-bit samples, mono, at sixteen
/// kilohertz (`scorsese_render::audio::speech`).
const BYTES_PER_SECOND: f64 = 32_000.0;

/// What was heard.
#[derive(Debug, Clone, PartialEq)]
pub struct Heard {
    /// The language, as Whisper detected it or was told.
    pub language: String,
    /// How long the recording is, in seconds.
    pub duration: f64,
    /// Every word, as written, with when it is said.
    pub words: Words,
}

/// Somewhere speech can be heard.
pub trait Transcriber {
    /// Hears `wav`, in `language` when one is forced.
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
    ) -> Result<Heard, TranscribeError>;
}

/// A language Whisper is told the speech is in: an ISO 639-1 code like `pt`,
/// or one of the few three-letter ones it knows (`haw`, `yue`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language(String);

impl Language {
    /// The code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Language {
    type Err = TranscribeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let code = text.trim().to_ascii_lowercase();
        if (2..=3).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_lowercase()) {
            Ok(Self(code))
        } else {
            Err(TranscribeError::Language(text.to_owned()))
        }
    }
}

/// The local Whisper server, as a [`Transcriber`].
#[derive(Debug, Clone)]
pub struct Whisper {
    api: Transcriptions,
    address: String,
}

impl Whisper {
    /// The server at `address` (`http://host:port`).
    pub fn new(address: &str) -> Self {
        Self {
            api: Transcriptions::new(address),
            address: address.to_owned(),
        }
    }
}

impl Transcriber for Whisper {
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
    ) -> Result<Heard, TranscribeError> {
        let within =
            SETTLING + Duration::from_secs_f64(wav.len() as f64 / BYTES_PER_SECOND * PER_SECOND);
        let request = Request {
            wav,
            model: MODEL,
            language: language.map(Language::as_str),
        };
        let reply = self
            .api
            .transcribe(&request, within)
            .map_err(|error| match error {
                HttpError::Unreachable { message, .. } => TranscribeError::NotRunning {
                    address: self.address.clone(),
                    why: message,
                },
                HttpError::Refused { status, body, .. } => {
                    TranscribeError::Refused { status, body }
                }
                HttpError::Unreadable { message, .. } => TranscribeError::Unreadable(message),
            })?;
        let words = reply
            .words
            .into_iter()
            .filter_map(|word| {
                let text = word.word.trim();
                (!text.is_empty()).then(|| Word {
                    text: text.to_owned(),
                    start: word.start,
                    end: word.end,
                })
            })
            .collect();
        Ok(Heard {
            language: reply.language,
            duration: reply.duration,
            words: Words { words },
        })
    }
}
