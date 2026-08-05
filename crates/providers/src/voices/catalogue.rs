//! What a voice catalogue is, from scorsese's side of the line.
//!
//! **Nothing here is a client.** A trait is what the caching and the expiry
//! handling above talk to, which is what lets every one of those behaviours be
//! driven by a test with no network and no key — including the ones that are
//! otherwise unreachable without somebody's billing details: a voice that has
//! been withdrawn, a voice the account may not use, and a key that is valid but
//! was never granted the scope to read a list.
//!
//! The vendor's own [`Filters`] is taken as-is rather than mirrored into a
//! scorsese-shaped one. Those five words — language, locale, gender, age,
//! accent — are ElevenLabs' vocabulary for narrowing a search, and a parallel
//! type would be five fields that exist only to be copied across, plus a place
//! for the two to disagree.

use serde::{Deserialize, Serialize};

use crate::api::elevenlabs::voices::Filters;

use super::error::VoiceError;

/// One voice, as a person picks one.
///
/// Flattened out of whichever listing it came from, because the two describe a
/// voice differently — one nests its traits under `labels`, the other spreads
/// them across top-level fields — and a reader comparing a built-in voice with
/// a Voice Library one should not have to know which is which.
///
/// Serialisable because this is exactly what gets cached: the cache is a list
/// of these and a note of when they were read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    /// The id a generation names. The one field that has to be right.
    pub id: String,
    /// What it is called, which is what a person picks by.
    pub name: String,
    /// Language, gender, age, accent, use case — whichever of them the listing
    /// filled in, in the order somebody reads them.
    #[serde(default)]
    pub traits: Vec<String>,
    /// A sentence about the voice, where whoever published it wrote one.
    #[serde(default)]
    pub description: Option<String>,
    /// An MP3 of the voice saying something. The one field here that answers
    /// the question a list cannot: *what does it sound like*.
    #[serde(default)]
    pub preview: Option<String>,
}

impl Voice {
    /// The one line a listing prints for this voice.
    ///
    /// Assembled here rather than in each of the places that print a voice —
    /// the CLI, the MCP tool, and whatever comes next — so that they cannot
    /// drift into describing the same voice differently.
    pub fn says(&self) -> String {
        let mut line = format!("{}  {}", self.id, self.name);
        if !self.traits.is_empty() {
            line.push_str(&format!("  ({})", self.traits.join(", ")));
        }
        line
    }
}

/// Why a voice cannot be used, when it cannot.
///
/// **Named for the outcome, not for the status code**, because more than one
/// thing makes a voice unusable and only one of them is deletion. A Voice
/// Library voice that exists perfectly well is refused to a free account per
/// voice, not merely per listing — so a type that only knew how to say *gone*
/// would report a plan limit as a withdrawn voice, and send somebody looking
/// for a replacement they do not need.
///
/// What is **not** here is *the provider could not be reached*. A timeout says
/// nothing whatever about the voice, and giving it a variant is how a flaky
/// connection would come to look like a withdrawn one — at which point
/// somebody's chosen narrator gets quietly replaced.
///
/// **This is a projection, not a second reading of the vendor.** What a refusal
/// means is decided once, in
/// [`Refusal`](crate::api::elevenlabs::refusal::Refusal), and these two
/// variants are the part of that answer which is a fact about a voice —
/// `VoiceGone` and `PlanRequired`. Working them out from the body again here
/// would be a second opinion about a sentence the vendor is already scheduled
/// to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unusable {
    /// Nothing at the provider answers to that id.
    ///
    /// Expected rather than exceptional: **every ElevenLabs default voice
    /// expires on 2026-12-31**, which is the single fact this whole module was
    /// built around.
    Gone,
    /// It exists, and this account is not allowed to use it.
    NotOnThisPlan {
        /// The vendor's own sentence, carried whole — it is the part that says
        /// which limit was hit.
        said: String,
    },
}

impl std::fmt::Display for Unusable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gone => f.write_str(
                "nothing there answers to it. Every ElevenLabs default voice expires on \
                 2026-12-31, so a voice going missing is expected rather than broken",
            ),
            Self::NotOnThisPlan { said } => write!(
                f,
                "the account may not use it: {said} Voice Library voices need a paid plan; \
                 the built-in voices do not"
            ),
        }
    }
}

/// Whether a voice id can actually be generated with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// It is there and usable, and this is it.
    Available(Voice),
    /// It is not something to generate with, and this is why.
    Unusable(Unusable),
}

/// Somewhere the voices can be listed.
pub trait Catalogue {
    /// The built-in voices — the small set an account has without doing
    /// anything. Unfiltered, because there are few enough to read.
    fn builtin(&self) -> Result<Vec<Voice>, VoiceError>;

    /// The Voice Library, narrowed by whatever was asked for.
    fn library(&self, filters: &Filters) -> Result<Vec<Voice>, VoiceError>;

    /// Whether one id can be generated with, and if not, why not.
    ///
    /// An [`Availability`] rather than a `Result` with a variant for it,
    /// because *this voice is gone* is an **answer to the question asked**, and
    /// the caller that wants it as a refusal gets one from
    /// [`require`](super::require). The distinction matters at the surface: a
    /// person checking an id wants to be told, and a generation about to spend
    /// money wants to be stopped.
    fn one(&self, voice_id: &str) -> Result<Availability, VoiceError>;

    /// What this catalogue is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
