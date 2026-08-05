//! A catalogue whose every answer was decided in advance.
//!
//! Scripted rather than clever, and it **counts calls** — which is how the
//! cache gets asserted at all: a cache that works is a cache that produces no
//! second call.

use std::cell::RefCell;

use scorsese_providers::video::ProviderError;
use scorsese_providers::voices::{Availability, Catalogue, Filters, More, Page, Voice, VoiceError};

/// A catalogue that answers exactly what it was told to.
pub(crate) struct Fake {
    /// What a listing answers with. An `Err` is produced fresh each time, so a
    /// failure can be scripted without the error type having to be cloneable.
    pub(crate) voices: Vec<Voice>,
    /// Set to make every listing fail as an unreachable provider does.
    pub(crate) unreachable: bool,
    /// Set to make the Voice Library answer the way a free plan is refused.
    pub(crate) free_plan: bool,
    /// Set to make a listing answer the way an unscoped key is refused.
    pub(crate) unscoped: bool,
    /// Set to make the Voice Library answer the way a capped search does:
    /// these voices, and a count of how many matched in all.
    pub(crate) more: Option<More>,
    /// What `one` answers with, for whichever id it is asked about.
    pub(crate) availability: Option<Availability>,
    /// How many times a listing has actually been asked for.
    pub(crate) listings: RefCell<usize>,
}

impl Fake {
    /// One that lists these voices and nothing else.
    pub(crate) fn listing(voices: Vec<Voice>) -> Self {
        Self {
            voices,
            unreachable: false,
            free_plan: false,
            unscoped: false,
            more: None,
            availability: None,
            listings: RefCell::new(0),
        }
    }

    /// One that answers about a single voice and lists nothing.
    pub(crate) fn answering(availability: Availability) -> Self {
        let mut fake = Self::listing(Vec::new());
        fake.availability = Some(availability);
        fake
    }

    /// How many times the network would have been touched.
    pub(crate) fn calls(&self) -> usize {
        *self.listings.borrow()
    }

    /// The scripted answer to a listing.
    fn answer(&self) -> Result<Vec<Voice>, VoiceError> {
        *self.listings.borrow_mut() += 1;
        if self.unreachable {
            return Err(VoiceError::Provider(ProviderError::new(
                "fake",
                "no route to host",
            )));
        }
        if self.unscoped {
            return Err(VoiceError::MissingPermission {
                said: String::from(
                    "The API key you used is missing the permission voices_read \
                     to execute this operation.",
                ),
            });
        }
        Ok(self.voices.clone())
    }
}

impl Catalogue for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        self.answer()
    }

    fn library(&self, _filters: &Filters) -> Result<Page, VoiceError> {
        if self.free_plan {
            *self.listings.borrow_mut() += 1;
            return Err(VoiceError::NoLibraryOnThisPlan {
                said: String::from("Free users cannot use library voices via the API."),
            });
        }
        Ok(Page {
            voices: self.answer()?,
            more: self.more,
        })
    }

    fn one(&self, _voice_id: &str) -> Result<Availability, VoiceError> {
        if self.unreachable {
            return Err(VoiceError::Provider(ProviderError::new(
                "fake",
                "no route to host",
            )));
        }
        self.availability
            .clone()
            .ok_or_else(|| VoiceError::Provider(ProviderError::new("fake", "nothing scripted")))
    }
}
