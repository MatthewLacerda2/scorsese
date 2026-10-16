//! What a video provider is, from scorsese's side of the line.
//!
//! **Three calls, because generation is long-running**, and that is the shape
//! the trait is built around rather than a detail of Veo's. Submitting hands
//! back a ticket in a moment; the work takes anywhere from ten seconds to
//! several hours; the download happens whenever somebody comes back for it.
//! Collapsing those into one blocking `generate(brief) -> bytes` would be a lie
//! about what happens, and the lie would cost the feature that matters most: a
//! generation surviving the process exiting.
//!
//! A provider that answers instantly — ElevenLabs, when it arrives — fits this
//! by returning [`Progress::Ready`] on the first poll. Designing for the easy
//! one first and widening it later is how the awkward one would have ended up
//! with a shape that cannot express it.
//!
//! **Nothing here is a client.** A trait is what the state machine in
//! [`super`] talks to, which is what lets the tests drive the whole lifecycle —
//! submit, wait, resume, fail — without a network or a cent. The repo rule is
//! that no test ever makes a real provider call, and this is the seam that
//! makes it true by construction rather than by anyone remembering.

use crate::api::http::HttpError;

use super::Brief;

/// The provider's name for work in flight.
///
/// Opaque on purpose: scorsese stores it in `project.json` and hands it back
/// later, and knowing nothing about its shape is what keeps it that. Veo's
/// happens to name the model it belongs to, which is why polling needs nothing
/// else remembered — but that is Veo's business, not this type's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket(pub String);

/// Where a finished generation can be fetched from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready(pub String);

/// What a poll found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// Still working. Nothing to do but come back.
    Waiting,
    /// Done, and there is something to fetch.
    Ready(Ready),
    /// Done, and it did not work. The string is the provider's own sentence,
    /// carried whole — it is usually the only part worth reading.
    Failed(String),
}

/// Why a call to a provider did not produce an answer.
///
/// Deliberately thin: the state machine above distinguishes *the provider
/// refused this brief* from *the network is down* by what it does next, not by
/// matching on a taxonomy this trait would have to keep in step with every
/// vendor. The one distinction carried is [`lost`](Self::lost), because it
/// decides whether asking again can ever help.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{provider}: {message}")]
pub struct ProviderError {
    /// Who was being called, for a message that names it.
    pub provider: String,
    /// What went wrong, in the provider's words where there are any.
    pub message: String,
    /// The provider will never answer for the ticket asked about — it does not
    /// know it, or this key may not see it. A run clears such a ticket as that
    /// one shot's outcome rather than stopping on it, because a ticket nobody
    /// can poll would otherwise stop every run in the project for ever (#1106).
    pub lost: bool,
}

impl ProviderError {
    /// A failure from `provider`, saying `message`.
    pub fn new(provider: impl Into<String>, message: impl std::fmt::Display) -> Self {
        Self {
            provider: provider.into(),
            message: message.to_string(),
            lost: false,
        }
    }

    /// A call to `provider` that failed in the transport, in the words every
    /// vendor's failure is put in ([`HttpError::explained`]): a 429 as a
    /// sentence rather than the vendor's page of JSON.
    pub fn http(provider: impl Into<String>, error: &HttpError) -> Self {
        Self::new(provider, error.explained(None))
    }

    /// A ticket `provider` will never answer for, saying `message`.
    pub fn lost(provider: impl Into<String>, message: impl std::fmt::Display) -> Self {
        Self {
            lost: true,
            ..Self::new(provider, message)
        }
    }
}

/// Somewhere a brief can be turned into a video.
pub trait VideoProvider {
    /// Hands a brief over and returns the ticket for it.
    ///
    /// Returns as soon as the request is accepted. The video does not exist yet
    /// and will not for a while — **this is the call that spends the money**,
    /// and everything after it is collection.
    fn submit(&self, brief: &Brief) -> Result<Ticket, ProviderError>;

    /// Asks whether a ticket has come good.
    fn poll(&self, ticket: &Ticket) -> Result<Progress, ProviderError>;

    /// Downloads a finished generation.
    fn fetch(&self, ready: &Ready) -> Result<Vec<u8>, ProviderError>;

    /// What this provider is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
