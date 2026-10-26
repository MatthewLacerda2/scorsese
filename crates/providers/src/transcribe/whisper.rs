//! The server behind a trait.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use scorsese_core::words::{Word, Words};

use crate::api::http::HttpError;
use crate::api::whisper::{Request, Transcriptions};

use super::{LONGEST, MODEL, TranscribeError};

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

/// How often a request being waited on looks for a stop.
const LISTEN_FOR_STOP: Duration = Duration::from_millis(100);

/// How long the server may take over `bytes` of speech: [`SETTLING`], and
/// [`PER_SECOND`] for each second of it up to [`LONGEST`] — so never past
/// about eight hours, whatever it is sent.
pub(super) fn within(bytes: usize) -> Duration {
    let seconds = (bytes as f64 / BYTES_PER_SECOND).min(LONGEST.as_secs_f64());
    SETTLING + Duration::from_secs_f64(seconds * PER_SECOND)
}

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
    /// Hears `wav`, in `language` when one is forced — or gives up waiting,
    /// [`TranscribeError::Stopped`], as soon as `stop` says so.
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
        stop: &dyn Fn() -> bool,
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
    /// The request runs on a thread of its own, waited on until it answers or
    /// `stop` says so. Stopped, the thread's request ends too, within a tenth
    /// of a second, whether its body is still going up or its reply is being
    /// waited on: the connection closed, the WAV dropped with the thread.
    /// What the server was already working out it finishes on its own two
    /// cores (`deploy/compose.yaml`).
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
        stop: &dyn Fn() -> bool,
    ) -> Result<Heard, TranscribeError> {
        let within = within(wav.len());
        let forced = language.map(|code| code.as_str().to_owned());
        let api = self.api.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let (answer, answered) = mpsc::channel();
        let sending = Arc::clone(&stopped);
        std::thread::spawn(move || {
            let request = Request {
                wav,
                model: MODEL,
                language: forced.as_deref(),
            };
            let _ = answer.send(api.transcribe(request, within, &sending));
        });
        let reply = loop {
            if stop() {
                stopped.store(true, Ordering::SeqCst);
                return Err(TranscribeError::Stopped);
            }
            match answered.recv_timeout(LISTEN_FOR_STOP) {
                Ok(reply) => break reply,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(TranscribeError::Unreadable(
                        "the request ended without an answer".to_owned(),
                    ));
                }
            }
        };
        let reply = reply.map_err(|error| match error {
            HttpError::Unreachable { message, .. } => TranscribeError::NotRunning {
                address: self.address.clone(),
                why: message,
            },
            HttpError::Refused { status, body, .. } => TranscribeError::Refused {
                status,
                body,
                forced: language.is_some(),
            },
            HttpError::Unreadable { message, .. } => TranscribeError::Unreadable(message),
        })?;
        // A stop that came with the answer still stops it.
        if stop() {
            return Err(TranscribeError::Stopped);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The wait grows with the speech and stops growing at the longest heard,
    /// however much is sent.
    #[test]
    fn the_wait_is_capped_at_the_longest_recording() {
        let minute = 60 * 32_000;
        assert_eq!(within(0), SETTLING);
        assert_eq!(within(minute), SETTLING + Duration::from_secs(240));
        let longest = SETTLING + Duration::from_secs(8 * 60 * 60);
        assert_eq!(within(120 * minute), longest);
        assert_eq!(within(usize::MAX), longest);
    }
}
