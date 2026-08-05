//! Veo, as three calls: submit, poll, fetch.
//!
//! The whole of scorsese's dealings with Google's video API. Nothing here knows
//! about the sketch lifecycle, the brief-hash cache, or where a file lands —
//! those are scorsese's business and live outside this directory. What this
//! answers is only *what does this vendor's API look like*.
//!
//! The three calls exist separately because generation is long-running: a
//! submit hands back a ticket in a moment, the work takes anywhere from ten
//! seconds to six hours, and the fetch happens whenever somebody comes back for
//! it. Collapsing them into one blocking call would be a lie about what the
//! vendor does.

pub mod request;
pub mod response;

use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// Every Veo endpoint hangs off this.
const BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// The header Google reads the key from.
const KEY_HEADER: &str = "x-goog-api-key";

/// The most a generated video is allowed to be, in bytes.
///
/// Eight seconds of 1080p is a few megabytes; this is orders above that, so it
/// bounds a redirect somewhere unexpected rather than any real video.
const MAX_VIDEO_BYTES: u64 = 512 * 1024 * 1024;

/// Which model a request is for.
///
/// The tiers scorsese offers, and the exact ids the API answers to. Confirmed
/// against the live `models` listing rather than taken from documentation —
/// these are `-preview` names and they will change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// The default tier.
    Fast,
    /// Cheaper, and without reference images.
    Lite,
}

impl Model {
    /// The id this tier is called in a URL.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Fast => "veo-3.1-fast-generate-preview",
            Self::Lite => "veo-3.1-lite-generate-preview",
        }
    }
}

/// Veo, reachable.
#[derive(Debug, Clone)]
pub struct Veo {
    caller: Caller,
}

impl Veo {
    /// A client that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// Hands a generation to the model and returns its ticket.
    ///
    /// Returns as soon as the request is accepted — the video does not exist
    /// yet and will not for a while. What comes back is the name to poll.
    pub fn submit(
        &self,
        model: Model,
        body: &request::Generate,
    ) -> Result<response::Submitted, HttpError> {
        let url = format!("{BASE}/models/{}:predictLongRunning", model.id());
        self.caller.post(&url, body)
    }

    /// Asks whether a ticket has come good.
    ///
    /// Takes the operation name exactly as it was handed back, which already
    /// contains the model it belongs to — so polling needs nothing else
    /// remembered about the request, which is what lets a ticket in
    /// `project.json` be the whole of what survives.
    pub fn poll(&self, operation: &str) -> Result<response::Operation, HttpError> {
        self.caller.get(&format!("{BASE}/{operation}"))
    }

    /// Downloads a finished video.
    ///
    /// The URI comes from the operation and is not public: it is a Files API
    /// link that needs the same key as a header, which is why this goes through
    /// the same caller rather than being a plain fetch.
    pub fn fetch(&self, uri: &str) -> Result<Vec<u8>, HttpError> {
        self.caller.download(uri, MAX_VIDEO_BYTES)
    }
}
