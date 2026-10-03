//! Designing a voice for a web user (#572): what `voice_design` does locally,
//! with the user's library and two tables standing in for the files it writes
//! beside `project.json`.
//!
//! Locally a design writes three MP3s into `generated/voice-design/<hash>/`
//! with a `design.json` beside them, and a kept voice is appended to
//! `designed-voices.json` (`scorsese_providers::voices::design`, *What it
//! creates is outside the project*). A stored project is only its document, and
//! the folder a tool call runs in is gone when the call answers — so here:
//!
//! - **The samples are library items**, each kept under a hash of its own
//!   ([`sample_hash`]: the design's brief hash and its place among the three),
//!   since a library holds one item per brief hash. They are the user's files,
//!   playable in the library and importable into any of their projects.
//! - **The design is a row of `voice_designs`** — its audit row as a paid
//!   generation (`credits::designs`), and once it worked the three candidates
//!   with the vendor's `generated_voice_id` each ([`find`], [`offering`]). Found again by
//!   the brief hash, so an unchanged design is never paid for twice, in any of
//!   the user's projects; a design whose samples were deleted from the library
//!   is not found, for the local reason — a hit is worth having only if the
//!   samples can be heard.
//! - **The ledger is `designed_voices`**, per user: a voice lives in the
//!   operator's ElevenLabs account and any project of theirs may name it, so
//!   it is the user's, not a project's. Each row points at the design that
//!   holds the description and the seed.
//!
//! ## Both verbs are jobs
//!
//! A design is paid for, and runs the way every paid generation runs here
//! (`crate::generations`): reserved in the transaction that enqueues it, then
//! a job asks ElevenLabs, keeps the samples and settles — charged when it
//! worked, released when it did not. A design takes seconds, so it could have
//! run inside the call; it does not, because a call interrupted between the
//! reservation and the answer would leave the money held with nothing to
//! settle it, where a job interrupted is run again, finds its open
//! reservation, and finishes. Keeping a candidate costs nothing but reaches
//! the same vendor, so it is a job too — one way to reach a vendor, one seam
//! ([`Vendors::studio`](crate::generations::Vendors::studio)) a test replaces.

mod job;
mod store;

pub use job::{design_handler, keep_handler};
pub use store::{Design, Kept, Sample, designed, find, get, offering};

use scorsese_core::hash_bytes;
use serde::{Deserialize, Serialize};

use crate::db::UserId;
use crate::library::{Item, Library, LibraryError};

/// The hash the `n`th sample (from 1) of the design with brief hash `brief`
/// is kept under in the library.
pub fn sample_hash(brief: &str, n: usize) -> String {
    hash_bytes(format!("voice-design-sample\n{brief}\n{n}\n").as_bytes())
}

/// What a design job carries: the brief, as it was quoted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPayload {
    /// `Brief::digest` — what the quote was bound to.
    pub brief: String,
    /// What the voice should be like.
    pub prompt: String,
    /// What the candidates read.
    pub passage: String,
    /// The vendor's best-effort determinism knob.
    pub seed: Option<u32>,
    /// How literally the description is followed.
    pub guidance: Option<f64>,
}

/// What a keep job carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepPayload {
    /// The design that offered the candidate.
    pub design: i64,
    /// The candidate's `generated_voice_id`.
    pub chosen: String,
    /// What to call the voice.
    pub name: String,
}

/// The library items holding `design`'s samples, in its order — `None` when
/// any of them is no longer in `user`'s library, so the design cannot be
/// heard and is not worth finding.
pub async fn samples(
    library: &Library,
    user: UserId,
    design: &Design,
) -> Result<Option<Vec<Item>>, LibraryError> {
    let mut items = Vec::with_capacity(design.candidates.len());
    for sample in design.candidates.iter() {
        match library.find_generated(user, &sample.sample).await? {
            Some(item) => items.push(item),
            None => return Ok(None),
        }
    }
    Ok(Some(items))
}
