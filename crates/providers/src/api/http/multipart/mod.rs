//! A `multipart/form-data` body, written out by hand (RFC 7578).
//!
//! Two calls take one: ElevenLabs' Instant Voice Cloning
//! (`POST /v1/voices/add`) wants its recordings as files in a form, not as
//! JSON, and so does the local Whisper server ([`whisper`](crate::api::whisper)).
//! `ureq` 3 has a multipart feature behind no flag scorsese turns on, and
//! turning one on for a single call buys a dependency `cargo deny` has to
//! keep clearing for what is fifty lines against a fixed specification — the
//! reason [`encoded`](super::encoded) is written here too.
//!
//! **A file's bytes are sent from where they are**, never copied into a
//! body: [`Form::body`] reads the parts in turn, with the length said up
//! front, so two hours of speech is one buffer, not three.
//!
//! **The boundary is derived from the content**, not drawn at random: a hash
//! of every part, so the same form is the same bytes (a test can hold them),
//! and checked against every part anyway — a boundary that occurred inside a
//! recording would cut it in two.

use std::borrow::Cow;
use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport,
};

use super::{Caller, HttpError};

/// How long a form may take to send and be answered — a few minutes of
/// somebody's recordings going up on a slow line, which the one-minute
/// [`TIMEOUT`](super::TIMEOUT) would cut off.
const FORM_WHOLE: Duration = Duration::from_secs(5 * 60);

impl Caller {
    /// POSTs `form` as `multipart/form-data` and reads the reply as JSON — for
    /// a vendor that takes files: ElevenLabs' voice cloning.
    pub fn post_multipart<R: DeserializeOwned>(
        &self,
        url: &str,
        form: &Form,
    ) -> Result<R, HttpError> {
        self.post_multipart_within(url, form, FORM_WHOLE)
    }

    /// [`Caller::post_multipart`], answered within `whole` rather than five
    /// minutes — for a server whose answer takes as long as the file it was
    /// sent: a transcription on a CPU.
    pub fn post_multipart_within<R: DeserializeOwned>(
        &self,
        url: &str,
        form: &Form,
        whole: Duration,
    ) -> Result<R, HttpError> {
        self.post_multipart_until(url, form, whole, &Arc::new(AtomicBool::new(false)))
    }

    /// [`Caller::post_multipart_within`], abandoned once `stopped` is set —
    /// while the body is sent or while its reply is waited on: the call
    /// returns, the connection is closed and the form is free to drop.
    pub fn post_multipart_until<R: DeserializeOwned>(
        &self,
        url: &str,
        form: &Form,
        whole: Duration,
        stopped: &Arc<AtomicBool>,
    ) -> Result<R, HttpError> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(whole))
            .http_status_as_error(false)
            .build();
        let agent = ureq::Agent::with_parts(
            config,
            DefaultConnector::new().chain(Stopping {
                stopped: Arc::clone(stopped),
            }),
            DefaultResolver::default(),
        );
        let (content_type, length, body) = form.body();
        let mut body = Stoppable {
            body,
            stopped: Arc::clone(stopped),
        };
        let sent = self
            .signed(agent.post(url))
            .header("Content-Type", content_type)
            .header("Content-Length", length.to_string())
            .send(ureq::SendBody::from_reader(&mut body));
        self.json(url, sent)
    }
}

/// How often a reply being waited on looks for a stop.
const LISTEN_FOR_STOP: Duration = Duration::from_millis(100);

/// Wraps each connection the agent opens in [`Stoppable`], so a stop reaches
/// a request blocked on its reply — which the body's own check cannot, once
/// the body is sent.
#[derive(Debug)]
struct Stopping {
    stopped: Arc<AtomicBool>,
}

impl Connector<Box<dyn Transport>> for Stopping {
    type Out = Stoppable<Box<dyn Transport>>;

    fn connect(
        &self,
        _: &ConnectionDetails,
        chained: Option<Box<dyn Transport>>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        Ok(chained.map(|inner| Stoppable {
            body: inner,
            stopped: Arc::clone(&self.stopped),
        }))
    }
}

/// Waits for input in slices of [`LISTEN_FOR_STOP`], looking for a stop
/// between them, rather than in one wait as long as the request's timeout.
impl Transport for Stoppable<Box<dyn Transport>> {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.body.buffers()
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        self.body.transmit_output(amount, timeout)
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        let until = timeout.not_zero().map(|wait| Instant::now() + *wait);
        loop {
            if self.stopped.load(Ordering::SeqCst) {
                return Err(ureq::Error::Io(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "stopped",
                )));
            }
            let left = until.map(|until| until.saturating_duration_since(Instant::now()));
            let slice = left.map_or(LISTEN_FOR_STOP, |left| left.min(LISTEN_FOR_STOP));
            let last = left.is_some_and(|left| left <= LISTEN_FOR_STOP);
            let waiting = NextTimeout {
                after: slice.into(),
                reason: timeout.reason,
            };
            match self.body.await_input(waiting) {
                Err(ureq::Error::Timeout(_)) if !last => {}
                done => return done,
            }
        }
    }

    fn is_open(&mut self) -> bool {
        self.body.is_open()
    }

    fn is_tls(&self) -> bool {
        self.body.is_tls()
    }
}

/// A body that stops being read once it is told to, so a request nobody
/// wants any more is not sent whole — or, around a connection, one that
/// stops waiting for its reply ([`Stopping`]).
#[derive(Debug)]
struct Stoppable<T> {
    body: T,
    stopped: Arc<AtomicBool>,
}

impl Read for Stoppable<Body<'_>> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "stopped"));
        }
        self.body.read(into)
    }
}

/// A form's body as it is sent: the framing it is written in, with each
/// part's bytes read from the form itself.
pub struct Body<'a> {
    pieces: Vec<Cow<'a, [u8]>>,
    piece: usize,
    at: usize,
}

impl Read for Body<'_> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        while let Some(piece) = self.pieces.get(self.piece) {
            let left = &piece[self.at..];
            if left.is_empty() {
                self.piece += 1;
                self.at = 0;
                continue;
            }
            let read = left.len().min(into.len());
            into[..read].copy_from_slice(&left[..read]);
            self.at += read;
            return Ok(read);
        }
        Ok(0)
    }
}

/// One part of a form.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    /// A text field.
    Text { name: String, value: String },
    /// A file: its field name, the file name sent with it, its media type and
    /// its bytes.
    File {
        name: String,
        file_name: String,
        media_type: String,
        bytes: Vec<u8>,
    },
}

/// A form being filled in, in the order its parts are sent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form {
    parts: Vec<Part>,
}

impl Form {
    /// An empty form.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same form, with the text field `name` set to `value`.
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.parts.push(Part::Text {
            name: name.to_owned(),
            value: value.into(),
        });
        self
    }

    /// The same form, with a file under the field `name`. A field may hold
    /// several files: each is its own part, under the same name.
    pub fn file(mut self, name: &str, file_name: &str, media_type: &str, bytes: Vec<u8>) -> Self {
        self.parts.push(Part::File {
            name: name.to_owned(),
            file_name: file_name.to_owned(),
            media_type: media_type.to_owned(),
            bytes,
        });
        self
    }

    /// The `Content-Type` header and the body, ready to send.
    pub fn encode(&self) -> (String, Vec<u8>) {
        let (content_type, length, mut body) = self.body();
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        body.read_to_end(&mut bytes)
            .expect("a body read from memory always reads");
        (content_type, bytes)
    }

    /// The `Content-Type` header, the body's length in bytes, and the body,
    /// each file read from the form rather than copied out of it.
    pub fn body(&self) -> (String, u64, Body<'_>) {
        let boundary = self.boundary();
        let mut pieces: Vec<Cow<'_, [u8]>> = Vec::new();
        let framing = |text: String| Cow::Owned(text.into_bytes());
        for part in &self.parts {
            match part {
                Part::Text { name, value } => {
                    pieces.push(framing(format!(
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n\
                         {value}\r\n",
                        quoted(name)
                    )));
                }
                Part::File {
                    name,
                    file_name,
                    media_type,
                    bytes,
                } => {
                    pieces.push(framing(format!(
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; \
                         filename=\"{}\"\r\nContent-Type: {media_type}\r\n\r\n",
                        quoted(name),
                        quoted(file_name)
                    )));
                    pieces.push(Cow::Borrowed(bytes));
                    pieces.push(Cow::Borrowed(b"\r\n"));
                }
            }
        }
        pieces.push(framing(format!("--{boundary}--\r\n")));
        let length = pieces.iter().map(|piece| piece.len() as u64).sum();
        (
            format!("multipart/form-data; boundary={boundary}"),
            length,
            Body {
                pieces,
                piece: 0,
                at: 0,
            },
        )
    }

    /// A boundary no part contains: a hash of every part, rehashed in the
    /// astronomically unlikely case one does.
    fn boundary(&self) -> String {
        let mut hasher = Sha256::new();
        for part in &self.parts {
            match part {
                Part::Text { name, value } => {
                    hasher.update(name.as_bytes());
                    hasher.update(value.as_bytes());
                }
                Part::File {
                    name,
                    file_name,
                    bytes,
                    ..
                } => {
                    hasher.update(name.as_bytes());
                    hasher.update(file_name.as_bytes());
                    hasher.update(bytes);
                }
            }
        }
        let mut digest = hasher.finalize().to_vec();
        loop {
            let boundary = format!("scorsese-{}", hex(&digest[..16]));
            if !self
                .parts
                .iter()
                .any(|part| part.contains(boundary.as_bytes()))
            {
                return boundary;
            }
            digest = Sha256::digest(&digest).to_vec();
        }
    }
}

impl Part {
    /// Whether `needle` occurs anywhere in what this part sends.
    fn contains(&self, needle: &[u8]) -> bool {
        let found = |haystack: &[u8]| haystack.windows(needle.len()).any(|at| at == needle);
        match self {
            Self::Text { name, value } => found(name.as_bytes()) || found(value.as_bytes()),
            Self::File {
                name,
                file_name,
                bytes,
                ..
            } => found(name.as_bytes()) || found(file_name.as_bytes()) || found(bytes),
        }
    }
}

/// `value` safe inside a quoted header parameter: a quote or a line break
/// would end the header early, so each is escaped as RFC 7578 §4.2 allows.
fn quoted(value: &str) -> String {
    value
        .replace('"', "%22")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
