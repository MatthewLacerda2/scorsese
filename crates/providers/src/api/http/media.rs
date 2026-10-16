//! Calls whose reply is the media itself, not JSON about it.
//!
//! ElevenLabs takes JSON and answers with an MP3 when it works and with JSON
//! when it does not, so the reply is read as bytes and the status decides
//! which of the two arrived (`bytes`, beside [`Caller`]).

use std::time::Duration;

use serde::Serialize;

use super::{Caller, HttpError, TIMEOUT};

impl Caller {
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
        self.post_bytes_within(url, body, TIMEOUT, limit)
    }

    /// [`post_bytes`](Caller::post_bytes), waiting up to `wait` for the whole
    /// reply rather than a minute — for media the vendor makes while the
    /// connection waits, as ElevenLabs composes a track of music (#1264).
    pub fn post_bytes_within<B: Serialize>(
        &self,
        url: &str,
        body: &B,
        wait: Duration,
        limit: u64,
    ) -> Result<Vec<u8>, HttpError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(wait))
            .http_status_as_error(false)
            .build()
            .into();
        let sent = self
            .signed(agent.post(url))
            .header("Content-Type", "application/json")
            .send_json(body);
        self.bytes(url, sent, limit)
    }
}
