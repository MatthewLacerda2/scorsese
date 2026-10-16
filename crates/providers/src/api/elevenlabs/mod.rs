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
//! **One call speaks, where Veo needs three to make a shot.** Speech is not
//! long-running: the request goes out and the MP3 comes back on the same
//! connection, so there is no ticket to remember, nothing to poll, and no
//! retention window to outlive. The asymmetry between this directory and
//! [`veo`](super::veo) is the asymmetry between the two vendors, and it is
//! worth leaving visible rather than smoothing over.
//!
//! [`Caller`]: crate::api::http::Caller

pub mod cloning;
pub mod design;
pub mod music;
pub mod refusal;
pub mod speech;
pub mod voices;

/// Every ElevenLabs endpoint hangs off this.
///
/// The version is part of it rather than part of each path, because `v1` is
/// what the whole API is: a vendor moving to `v2` moves all of it at once, and
/// this is then the one line to change.
const BASE: &str = "https://api.elevenlabs.io/v1";

/// The header ElevenLabs reads the key from.
const KEY_HEADER: &str = "xi-api-key";

/// The one output format scorsese asks for, for speech and for a voice
/// design's samples alike.
///
/// The API's own default, and not gated behind any subscription tier — which is
/// the whole reason it is a constant rather than a choice. 192kbps MP3 needs
/// Creator, PCM and WAV at 44.1kHz need Pro, and a picker whose options depend
/// on somebody's plan is a tier-detection feature wearing a dropdown.
///
/// It is also a line of every ElevenLabs brief's fingerprint, read from here,
/// so changing it asks for every line and design again rather than serving the
/// old format's files as the new one's.
pub const OUTPUT_FORMAT: &str = "mp3_44100_128";
