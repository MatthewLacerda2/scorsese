//! Where a `voice_id` comes from, so nobody has to paste one.
//!
//! A narration asset names the voice that reads it, and that id has to come
//! from somewhere. **Not from a constant in this repository**: every
//! ElevenLabs default voice expires on 2026-12-31 and the default set is being
//! replaced before then, so a hardcoded id is not a shortcut with a small cost
//! — it is a guaranteed outage with a date already on it. The list is resolved
//! at runtime, and that is the whole reason this module exists.
//!
//! # Four things hold this together
//!
//! **A list is cached, so browsing is not a round trip.** Both listings land
//! under the project's `cache/` — rebuildable, gitignored, never part of what
//! `scp -r` has to carry — which is also what lets an offline `scorsese
//! voices` still answer. A cache that never expired would be the failure this
//! module is about, so a stale one refreshes itself and every answer says how
//! old it is. See [`cache`].
//!
//! **A voice that cannot be used is a named state, not a crash.**
//! [`Availability`] is what a lookup answers with, [`Unusable`] says which
//! reason it was, and [`require`] is the one call that turns either into a
//! refusal a person can act on. What none of them does is fall back to some
//! other voice: that would spend money producing narration in a voice nobody
//! chose, and the recording would sound fine.
//!
//! **Not reachable is not the same as not there.** A 404 says the voice is
//! gone; a timeout says nothing about the voice at all. Collapsing them would
//! mean a flaky network deleted somebody's voice choice, so [`Unusable`] has no
//! variant for it — an unreachable provider is a
//! [`VoiceError::Provider`] and stays one.
//!
//! **A refusal is read, not just counted.** ElevenLabs answers `401` both for a
//! key that is wrong and for a key that is right but was never granted
//! `voices_read`, and those are fixed in different places. The body carries the
//! word that tells them apart, so the body is what gets read — see
//! [`refusal`](crate::api::elevenlabs::refusal).
//!
//! # The trait is the test seam
//!
//! [`Catalogue`] is what everything above talks to, so the whole of this —
//! cache hit, refresh, expiry, the plan refusal, the missing scope — is
//! exercised without a network or a key. The repo rule against real provider
//! calls in tests is satisfied by construction rather than by anyone
//! remembering.

mod cache;
mod catalogue;
mod elevenlabs;
mod error;
mod run;

pub use cache::REFRESH_AFTER_DAYS;
pub use catalogue::{Availability, Catalogue, Unusable, Voice};
pub use elevenlabs::ElevenLabsVoices;
pub use error::VoiceError;
pub use run::{Answer, Freshness, builtin, library, require};

pub use crate::api::elevenlabs::voices::{DEFAULT_PAGE_SIZE, Filters, MAX_PAGE_SIZE};
