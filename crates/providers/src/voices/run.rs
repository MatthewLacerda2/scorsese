//! Asking for a list, and asking after one voice.
//!
//! The verbs, and the three decisions they make that nothing below them can.
//!
//! **When to go to the network.** A fresh cache answers; a stale one is
//! refreshed; a caller who says `refresh` refreshes regardless. A vendor that
//! cannot be reached falls back to whatever is cached rather than failing,
//! because a week-old list is a far better answer to *which voices are there*
//! than no answer at all — and every reply says which of the two it is, so
//! nobody mistakes one for the other.
//!
//! **What gets written down.** Only a list that actually came back. A refusal
//! never overwrites a cache entry, so a key that lost a permission this morning
//! does not also cost somebody the list they had yesterday.
//!
//! **What a missing voice means.** [`require`] is the whole of the 404
//! recovery: it turns *unusable* into a refusal naming the id, the reason and
//! what to do next, and it does not, ever, substitute another voice.

use std::path::Path;

use scorsese_core::Timestamp;

use crate::api::elevenlabs::voices::{Filters, MAX_PAGE_SIZE};

use super::cache;
use super::catalogue::{Availability, Catalogue, Voice};
use super::error::VoiceError;

/// Where a list came from.
///
/// Carried with every answer because *these are the voices* and *these were the
/// voices when anyone could last ask* are different claims, and the second one
/// is what somebody debugging a voice that will not generate actually needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// Read from the provider on this call.
    Fetched,
    /// Served out of `cache/`.
    Cached {
        /// How old it is, where that could be worked out.
        days: Option<i64>,
        /// Why the provider was not asked: `None` when the cache was simply
        /// current, and the failure itself when it was tried and would not
        /// answer.
        refusal: Option<String>,
    },
}

/// A list, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The voices.
    pub voices: Vec<Voice>,
    /// Fetched, or out of the cache and how old.
    pub freshness: Freshness,
}

/// The built-in voices.
pub fn builtin(
    root: &Path,
    catalogue: &dyn Catalogue,
    refresh: bool,
) -> Result<Answer, VoiceError> {
    listed(root, "builtin", refresh, || catalogue.builtin())
}

/// The Voice Library, narrowed by `filters`.
///
/// The filters are part of the cache key, so two searches never overwrite each
/// other's answer — a search for Portuguese voices and a search for old British
/// ones are two lists, not one list twice.
pub fn library(
    root: &Path,
    catalogue: &dyn Catalogue,
    filters: &Filters,
    refresh: bool,
) -> Result<Answer, VoiceError> {
    if let Some(asked) = filters.page_size.filter(|size| *size > MAX_PAGE_SIZE) {
        return Err(VoiceError::PageTooLarge { asked });
    }
    let key = format!("library{}", filters.query());
    listed(root, &key, refresh, || catalogue.library(filters))
}

/// The voice `voice_id` names, or a refusal that says what to do instead.
///
/// **Never cached, and that is deliberate.** This asks whether a voice somebody
/// already chose can still be generated with, so answering it out of a saved
/// list would be answering with the very information that has gone out of date.
///
/// This is the call a generation makes before spending anything. What it must
/// never do is return some other voice.
pub fn require(catalogue: &dyn Catalogue, voice_id: &str) -> Result<Voice, VoiceError> {
    match catalogue.one(voice_id)? {
        Availability::Available(voice) => Ok(voice),
        Availability::Unusable(why) => Err(VoiceError::Unusable {
            voice_id: voice_id.to_owned(),
            provider: catalogue.name(),
            why,
        }),
    }
}

/// The cache-or-fetch decision, which is the same for both listings.
///
/// `fetch` is a closure rather than a second trait method so that the ordering
/// — cache first, network second, cache again on failure — is written once. Two
/// copies of it would be two places for the fallback to be forgotten, and the
/// one that forgot it would be the one somebody runs on a train.
fn listed(
    root: &Path,
    key: &str,
    refresh: bool,
    fetch: impl FnOnce() -> Result<Vec<Voice>, VoiceError>,
) -> Result<Answer, VoiceError> {
    let now = Timestamp::unix_now();
    let cached = cache::read(root, key);
    if !refresh && let Some(entry) = cached.as_ref().filter(|entry| !entry.is_stale(now)) {
        return Ok(Answer {
            voices: entry.voices.clone(),
            freshness: Freshness::Cached {
                days: entry.age_days(now),
                refusal: None,
            },
        });
    }

    match fetch() {
        Ok(voices) => {
            // A cache that will not be written is not worth failing a listing
            // over: the voices are in hand, and the only cost is asking again.
            let _ = cache::write(root, key, &voices);
            Ok(Answer {
                voices,
                freshness: Freshness::Fetched,
            })
        }
        // Better a week-old list than none. The reply says so outright rather
        // than passing it off as current.
        Err(error) => match cached {
            Some(entry) => Ok(Answer {
                voices: entry.voices.clone(),
                freshness: Freshness::Cached {
                    days: entry.age_days(now),
                    refusal: Some(error.to_string()),
                },
            }),
            None => Err(error),
        },
    }
}
