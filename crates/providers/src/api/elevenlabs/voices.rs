//! The two voice listings, and the lookup of a single voice.
//!
//! Three endpoints, and they are not interchangeable. `/voices` is the small
//! built-in set an account already has; `/shared-voices` is the Voice Library,
//! which is thousands of voices other people published and is the only one of
//! the two worth filtering. `/voices/{id}` answers about exactly one, and is
//! how *"is this id still a voice?"* gets asked — the question that matters,
//! because **every ElevenLabs default voice expires on 2026-12-31**.
//!
//! Both listings answer with the same envelope — `{"voices": [...]}` — so one
//! [`Listing`] reads both. The entries differ in which fields are filled in
//! rather than in shape, which is why [`Voice`] names the union of them and
//! defaults every field but the two that identify it.

use serde::Deserialize;
use std::collections::BTreeMap;

use super::{BASE, KEY_HEADER};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The most voices the vendor will put on one page of the Voice Library.
///
/// Asking for more is refused by ElevenLabs rather than truncated, so it is
/// worth refusing locally: a round trip to be told a number is too big teaches
/// nothing the number itself does not.
pub const MAX_PAGE_SIZE: u32 = 100;

/// How many the vendor returns when nobody says.
///
/// Written down rather than left implicit, because a caller reading a list of
/// thirty needs to know whether that is all there is or all that was asked for.
pub const DEFAULT_PAGE_SIZE: u32 = 30;

/// What to narrow the Voice Library down to.
///
/// The vendor's own query parameters, spelled the way the vendor spells them,
/// for the reason the rest of this directory exists: this file should be
/// checkable against ElevenLabs' reference page line by line. Every one is
/// optional and an absent one is simply not sent — a filter with an empty
/// value is not the same request as no filter, and sending one would quietly
/// narrow a search nobody narrowed.
///
/// `language` is the field this whole listing was worth building for. It is
/// what surfaces actual pt-BR speakers instead of an English voice reading
/// Portuguese.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    /// An ISO 639-1 code: `pt`, `en`, `es`.
    pub language: Option<String>,
    /// A finer regional code where the vendor has one: `pt-br`, `en-us`.
    pub locale: Option<String>,
    /// `male`, `female`, `neutral`.
    pub gender: Option<String>,
    /// `young`, `middle_aged`, `old`.
    pub age: Option<String>,
    /// Free text the vendor matches loosely: `british`, `brazilian`.
    pub accent: Option<String>,
    /// How many to return, at most [`MAX_PAGE_SIZE`]. Absent means the
    /// vendor's own [`DEFAULT_PAGE_SIZE`].
    pub page_size: Option<u32>,
}

impl Filters {
    /// These filters as the query string to hang off the URL.
    ///
    /// Built here rather than at the call site so that "which parameters exist"
    /// is answered by the struct above and nowhere else. Values are
    /// percent-encoded because an accent is free text and a space in one would
    /// otherwise produce a URL the transport refuses.
    pub fn query(&self) -> String {
        let mut pairs: Vec<String> = Vec::new();
        let mut text = |name: &str, value: &Option<String>| {
            if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
                pairs.push(format!("{name}={}", encoded(value.trim())));
            }
        };
        text("language", &self.language);
        text("locale", &self.locale);
        text("gender", &self.gender);
        text("age", &self.age);
        text("accent", &self.accent);
        if let Some(size) = self.page_size {
            pairs.push(format!("page_size={size}"));
        }
        if pairs.is_empty() {
            String::new()
        } else {
            format!("?{}", pairs.join("&"))
        }
    }
}

/// The voice listings, reachable.
///
/// Its own client rather than a method on a vendor-wide one, because a listing
/// is the half of ElevenLabs that costs nothing: reading it needs a key but
/// spends no credits, and keeping it separate from the half that does spend
/// them makes that visible in the type a caller holds.
#[derive(Debug, Clone)]
pub struct Voices {
    caller: Caller,
}

impl Voices {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// The built-in voices an account already has.
    ///
    /// `category=premade` on purpose: without it the reply also carries voices
    /// the account cloned or designed, and this listing exists to answer *what
    /// can I pick with no setup at all*.
    pub fn premade(&self) -> Result<Listing, HttpError> {
        self.caller.get(&format!("{BASE}/voices?category=premade"))
    }

    /// The Voice Library, narrowed by whatever was asked for.
    ///
    /// **Not available through the API on a free plan.** The refusal that
    /// produces is a fact about somebody's subscription rather than about their
    /// request, and reading it as one is [`crate::voices`]'s job — this call
    /// reports what the vendor said and nothing more.
    pub fn shared(&self, filters: &Filters) -> Result<Listing, HttpError> {
        self.caller
            .get(&format!("{BASE}/shared-voices{}", filters.query()))
    }

    /// One voice, by id.
    ///
    /// A 404 here is the answer this endpoint exists for, and it arrives as
    /// [`HttpError::Refused`] with that status rather than as anything special:
    /// deciding that *no such voice* is recoverable and *no route to host* is
    /// not is a judgement above this layer, and making it here would leave the
    /// two indistinguishable to everybody else.
    pub fn one(&self, voice_id: &str) -> Result<Voice, HttpError> {
        self.caller
            .get(&format!("{BASE}/voices/{}", encoded(voice_id)))
    }
}

/// A page of voices, from either listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Listing {
    /// The voices on this page. Defaulted, so a reply carrying an empty
    /// envelope reads as no voices rather than as an unparseable answer.
    #[serde(default)]
    pub voices: Vec<Voice>,
    /// Whether the vendor has more beyond this page.
    #[serde(default)]
    pub has_more: bool,
    /// How many voices match the search in all, where the vendor counts them.
    ///
    /// `/shared-voices` sends it — the captured fixture answers `4274` to a
    /// search for Portuguese — and `/voices` does not. An `Option` rather than
    /// a defaulted zero for exactly that reason: *no count was sent* and *none
    /// matched* are different answers, and only one of them is worth printing.
    #[serde(default)]
    pub total_count: Option<u32>,
}

/// One voice, as either listing describes it.
///
/// Read permissively, like every reply in this directory: only the fields
/// scorsese acts on are named and unknown ones are ignored, because a request
/// is ours to get exactly right and a reply is somebody else's to change.
///
/// The two listings fill in different halves of this. `/voices` puts an
/// accent and an age inside `labels`; `/shared-voices` puts them at the top
/// level and adds a language. Naming the union rather than two types is what
/// lets one listing be shown next to the other.
///
/// **Checked against real response bodies**, not only against the reference
/// page — both listings are captured under `fixtures/elevenlabs/` and read by
/// the tests in [`crate::voices`]. That is what caught `descriptive`, which
/// the reference page does not list among the top-level fields but which
/// `/shared-voices` sends for every voice.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Voice {
    /// The id a generation names. The only field here that is load-bearing.
    #[serde(default)]
    pub voice_id: String,
    /// What the voice is called, which is what a person picks by.
    #[serde(default)]
    pub name: String,
    /// `premade`, `professional`, `cloned`, `generated`.
    #[serde(default)]
    pub category: Option<String>,
    /// A sentence about the voice, where whoever published it wrote one.
    #[serde(default)]
    pub description: Option<String>,
    /// `/voices`' way of carrying accent, age, gender and use case.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// `/shared-voices` only: an ISO 639-1 code.
    #[serde(default)]
    pub language: Option<String>,
    /// `/shared-voices` only.
    #[serde(default)]
    pub accent: Option<String>,
    /// `/shared-voices` only.
    #[serde(default)]
    pub age: Option<String>,
    /// `/shared-voices` only.
    #[serde(default)]
    pub gender: Option<String>,
    /// `/shared-voices` only: what the voice is good for, in one word.
    #[serde(default)]
    pub use_case: Option<String>,
    /// `/shared-voices` only: what the voice *sounds like*, in one word —
    /// `calm`, `casual`, `confident`.
    ///
    /// The same fact `/voices` carries as a `descriptive` label, and the one
    /// this file was wrong about: it is a top-level field on the Voice Library
    /// listing, so reading it only out of `labels` lost it for every voice
    /// there while every built-in voice kept it.
    #[serde(default)]
    pub descriptive: Option<String>,
    /// An MP3 of the voice saying something, for a person who wants to hear it
    /// before spending anything on it.
    #[serde(default)]
    pub preview_url: Option<String>,
}

/// Everything RFC 3986 lets stand unescaped in a query value.
const UNRESERVED: &[u8] = b"-._~";

/// `value`, safe to put in a URL.
///
/// Written here rather than taken as a dependency, for the reason the MCP
/// server gives for its own base64: a dozen lines against a fixed
/// specification, and every dependency is one `cargo deny` has to keep
/// clearing. Deliberately strict — everything outside the unreserved set is
/// escaped, including characters a query would tolerate, because being
/// over-cautious in a URL costs nothing and being wrong costs a failed call.
fn encoded(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED.contains(byte) {
            out.push(char::from(*byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_filters_is_no_query_string() {
        assert_eq!(Filters::default().query(), "");
    }

    #[test]
    fn only_the_filters_that_were_asked_for_are_sent() {
        let filters = Filters {
            language: Some(String::from("pt")),
            page_size: Some(50),
            ..Filters::default()
        };
        assert_eq!(filters.query(), "?language=pt&page_size=50");
    }

    /// An empty string is somebody not filtering, and sending it as a filter
    /// would narrow a search to nothing at all.
    #[test]
    fn a_blank_filter_is_not_a_filter() {
        let filters = Filters {
            accent: Some(String::from("  ")),
            ..Filters::default()
        };
        assert_eq!(filters.query(), "");
    }

    #[test]
    fn a_value_with_a_space_in_it_survives_the_url() {
        let filters = Filters {
            accent: Some(String::from("latin american")),
            ..Filters::default()
        };
        assert_eq!(filters.query(), "?accent=latin%20american");
    }
}
