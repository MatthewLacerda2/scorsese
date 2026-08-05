//! ElevenLabs, as the calls scorsese actually makes.
//!
//! The second vendor, and it needed no transport work at all: [`Caller`] takes
//! the header a key travels in, so `xi-api-key` is an argument rather than a
//! branch. That was the point of writing it that way, and this directory is the
//! evidence.
//!
//! Nothing here knows about the sketch lifecycle, the brief-hash cache, what a
//! generation costs, or where a file lands. Those are scorsese's business and
//! live beside this directory rather than in it. A module here answers exactly
//! one question — *what does this vendor's API look like* — so that it can be
//! read with the vendor's own reference page open next to it.
//!
//! [`Caller`]: crate::api::http::Caller

pub mod refusal;
pub mod voices;

/// Every ElevenLabs endpoint hangs off this.
///
/// The version is part of it rather than part of each path, because `v1` is
/// what the whole API is: a vendor moving to `v2` moves all of it at once, and
/// this is then the one line to change.
const BASE: &str = "https://api.elevenlabs.io/v1";

/// The header ElevenLabs reads the key from.
const KEY_HEADER: &str = "xi-api-key";
