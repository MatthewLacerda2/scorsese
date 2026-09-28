//! The one place an HTTP library is named.
//!
//! Rust's standard library cannot make an HTTP request, so one dependency has
//! to do it. Which one is behind this module and nowhere else: a vendor module
//! declares its endpoints and its payloads and hands them here, so replacing
//! the transport is one file's work and "how does scorsese talk to a network"
//! is one file's reading.
//!
//! Blocking, and deliberately. Nothing in scorsese is asynchronous — the CLI,
//! the MCP server and the window are all straight-line code — and a generation
//! is polled *seconds* apart, not milliseconds. An async runtime would be a
//! larger change to this codebase than the feature that asked for it.

use std::io::{BufReader, Read};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// How long to wait on a single request before giving up on it.
///
/// Generous, because the slow part of generating a video is not this: a submit
/// returns a ticket immediately and a poll answers immediately. What this
/// bounds is a request that has genuinely hung, and the caller retries or
/// resumes rather than dying.
const TIMEOUT: Duration = Duration::from_secs(60);

/// How long a streamed reply may take to arrive whole.
///
/// A stream is the answer to a reply that takes minutes — a model thinking
/// and writing tens of thousands of tokens — so the one-minute bound above
/// would cut every long one off. What bounds a stream instead is this, and
/// [`STREAM_FIRST_BYTE`] for a server that never starts answering.
const STREAM_WHOLE: Duration = Duration::from_secs(30 * 60);

/// How long a streamed reply may take to start: its headers.
const STREAM_FIRST_BYTE: Duration = Duration::from_secs(120);

/// Why a call did not produce an answer.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// The request never completed: no network, DNS, TLS, a timeout.
    #[error("could not reach {url}: {message}")]
    Unreachable {
        /// What was being called.
        url: String,
        /// What the transport said.
        message: String,
    },

    /// The server answered, and the answer was a refusal.
    ///
    /// The body is carried whole rather than summarised, because a vendor puts
    /// the useful sentence in it — *"1080p is not supported for a duration of 4
    /// seconds"* is the API telling you exactly what to change, and throwing it
    /// away to report "400 Bad Request" would be discarding the only part worth
    /// reading.
    #[error("{url} answered {status}: {body}")]
    Refused {
        /// What was being called.
        url: String,
        /// The status code.
        status: u16,
        /// Whatever the server said, verbatim.
        body: String,
    },

    /// The answer arrived and was not the shape we expect.
    #[error("{url} answered with something unreadable: {message}")]
    Unreadable {
        /// What was being called.
        url: String,
        /// What the parser said.
        message: String,
    },
}

/// A caller that can reach a vendor's API.
///
/// Carries the credential rather than taking it per call, so a key is put in
/// one place and every request made through this one is authenticated the same
/// way. It is a [`Secret`](crate::credentials::Secret) at the boundary and a
/// header from here on.
#[derive(Debug, Clone)]
pub struct Caller {
    header: &'static str,
    key: String,
    /// Headers every request carries besides the key — a vendor's API
    /// version, a feature it gates behind a header.
    extra: Vec<(&'static str, String)>,
}

impl Caller {
    /// A caller that sends `key` in `header` on every request.
    pub fn new(header: &'static str, key: &crate::credentials::Secret) -> Self {
        Self {
            header,
            key: key.expose().to_owned(),
            extra: Vec::new(),
        }
    }

    /// The same caller, also sending `value` in `header` on every request.
    pub fn with(mut self, header: &'static str, value: impl Into<String>) -> Self {
        self.extra.push((header, value.into()));
        self
    }

    /// POSTs `body` as JSON and hands back the reply as it arrives — for a
    /// vendor that streams its answer (server-sent events) rather than
    /// sending it whole.
    ///
    /// A refusal is read whole and returned as one, exactly as [`post`]
    /// returns it: a vendor that refuses does not stream the refusal.
    ///
    /// [`post`]: Caller::post
    pub fn post_stream<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<BufReader<Box<dyn Read + Send>>, HttpError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(TIMEOUT))
            .timeout_send_body(Some(TIMEOUT))
            .timeout_recv_response(Some(STREAM_FIRST_BYTE))
            .timeout_recv_body(Some(STREAM_WHOLE))
            .http_status_as_error(false)
            .build()
            .into();
        let sent = self
            .signed(agent.post(url))
            .header("Content-Type", "application/json")
            .send_json(body);
        let response = refused(url, sent)?;
        let body: Box<dyn Read + Send> = Box::new(response.into_body().into_reader());
        Ok(BufReader::new(body))
    }

    /// `request`, carrying the key and every extra header.
    fn signed<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        self.extra.iter().fold(
            request.header(self.header, &self.key),
            |request, (header, value)| request.header(*header, value),
        )
    }

    /// POSTs `body` as JSON and reads the reply as JSON.
    pub fn post<B: Serialize, R: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<R, HttpError> {
        let sent = self
            .signed(self.agent().post(url))
            .header("Content-Type", "application/json")
            .send_json(body);
        read_json(url, sent)
    }

    /// POSTs `body` as JSON and reads the reply as bytes — for a vendor that
    /// answers with the media itself.
    ///
    /// A third shape rather than a variation on the other two, because the
    /// asymmetry is the vendor's: ElevenLabs takes JSON and answers with an MP3
    /// when it works and with JSON when it does not. So the reply cannot be
    /// typed in advance, and the status is what decides which of the two
    /// arrived — a refusal keeps its body whole, which is where the vendor
    /// writes the sentence naming the permission or the plan that was missing.
    pub fn post_bytes<B: Serialize>(
        &self,
        url: &str,
        body: &B,
        limit: u64,
    ) -> Result<Vec<u8>, HttpError> {
        let sent = self
            .signed(self.agent().post(url))
            .header("Content-Type", "application/json")
            .send_json(body);
        read_bytes(url, refused(url, sent)?, limit)
    }

    /// GETs `url` and reads the reply as JSON.
    pub fn get<R: DeserializeOwned>(&self, url: &str) -> Result<R, HttpError> {
        let sent = self.signed(self.agent().get(url)).call();
        read_json(url, sent)
    }

    /// GETs `url` and reads the reply as bytes — for the media itself.
    ///
    /// Bounded rather than read to whatever arrives: a reply claiming to be
    /// larger than any video we asked for is a redirect to somewhere unexpected
    /// or a server having a bad day, and neither is worth filling memory over.
    pub fn download(&self, url: &str, limit: u64) -> Result<Vec<u8>, HttpError> {
        let sent = self.signed(self.agent().get(url)).call();
        read_bytes(url, refused(url, sent)?, limit)
    }

    /// The transport, configured to hand back a refusal rather than throw it
    /// away.
    ///
    /// `http_status_as_error(false)` is the load-bearing setting: by default a
    /// 4xx becomes an error carrying only its number, and the body — which is
    /// where the vendor writes the sentence worth reading — is discarded. We
    /// want the sentence, so the status is ours to check.
    fn agent(&self) -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into()
    }
}

/// A reply's body, up to `limit` bytes.
///
/// Bounded rather than read to whatever arrives: a reply claiming to be larger
/// than any media we asked for is a redirect somewhere unexpected or a server
/// having a bad day, and neither is worth filling memory over.
fn read_bytes(
    url: &str,
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
) -> Result<Vec<u8>, HttpError> {
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| HttpError::Unreadable {
            url: url.to_owned(),
            message: error.to_string(),
        })?;
    Ok(bytes)
}

/// The reply as `R`, or the most useful account of why there is not one.
fn read_json<R: DeserializeOwned>(
    url: &str,
    sent: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<R, HttpError> {
    let mut response = refused(url, sent)?;
    response
        .body_mut()
        .read_json()
        .map_err(|error| HttpError::Unreadable {
            url: url.to_owned(),
            message: error.to_string(),
        })
}

/// The response, unless it never arrived or arrived as a refusal.
///
/// A refusal keeps its body whole. The vendor puts the useful sentence there —
/// *"1080p is not supported for a duration of 4 seconds"* is the API saying
/// exactly what to change — and reducing that to "400" would discard the only
/// part worth reading.
fn refused(
    url: &str,
    sent: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<ureq::http::Response<ureq::Body>, HttpError> {
    let mut response = sent.map_err(|error| HttpError::Unreachable {
        url: url.to_owned(),
        message: error.to_string(),
    })?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    let body = response
        .body_mut()
        .read_to_string()
        .unwrap_or_else(|error| format!("(unreadable body: {error})"));
    Err(HttpError::Refused {
        url: url.to_owned(),
        status,
        body,
    })
}
