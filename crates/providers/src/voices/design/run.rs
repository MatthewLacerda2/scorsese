//! Designing a voice, and keeping one — the two verbs, and the decisions they
//! make that nothing below them can.
//!
//! **When money is spent.** [`design`] checks the samples on disk before it
//! checks anything else: a design already paid for is answered from
//! `generated/`, whatever anyone claims about it. Only a genuine miss reaches
//! the ceiling check, and only a run that passes the ceiling reaches the
//! vendor. [`keep`] never spends at all — the candidate was paid for when it
//! was designed — which is why it is a separate verb and not a flag.
//!
//! **What gets written down, and when.** The samples and their record are
//! written the instant they arrive, before anything can go wrong with them, and
//! then the line in `generations.json` that says the design was paid for
//! (#1486); the designed-voices entry is written the instant the voice exists. Each is fatal if it
//! fail, because both are the record of money already spent — and the failure
//! mode of being relaxed about it is paying twice for the same three samples,
//! or holding a voice id whose description nobody can recover.
//!
//! **What a repeat costs.** Nothing. Asking for the same design twice is a
//! cache hit; keeping the same candidate twice creates a second voice at the
//! vendor, which is a real effect and so is reported rather than deduplicated —
//! two voices from one candidate is somebody's decision, not an accident this
//! layer should quietly undo.
//!
//! **What undoes a keep.** [`forget`] deletes a kept voice at the vendor and
//! marks it so in the ledger (#1381); the design behind it stays.

use std::path::Path;

use scorsese_core::Timestamp;

use crate::credentials::Budget;
use crate::spending;

use super::super::Voice;
use super::brief::Brief;
use super::error::DesignError;
use super::ledger::{self, Designed};
use super::price::{self, Estimate};
use super::session::{self, Session};
use super::studio::Studio;

/// A design, and whether it cost anything this time.
#[derive(Debug, Clone, PartialEq)]
pub struct Designing {
    /// The three candidates and what made them.
    pub session: Session,
    /// What it was estimated to cost. **Our arithmetic, never a bill.**
    pub estimate: Estimate,
    /// Whether the samples were already on disk.
    ///
    /// Carried rather than inferred, because *this cost five cents* and *this
    /// cost five cents at some point and nothing today* are different claims,
    /// and a surface that could not tell them apart would report a spend every
    /// time somebody listened again.
    pub already_paid: bool,
}

impl Designing {
    /// What this run actually spent, which is nothing when it was cached.
    pub fn spent_cents(&self) -> u64 {
        if self.already_paid {
            0
        } else {
            self.estimate.cents
        }
    }
}

/// Three candidates for this brief. **The half that spends money.**
///
/// `budget` is checked before the call and not after, because after is a
/// receipt rather than a decision. It is the same ceiling a generated shot is
/// held to and it is not negotiable by a flag — an agent working overnight
/// cannot be asked a question, so it is given a number it may not cross.
pub fn design(
    root: &Path,
    studio: &dyn Studio,
    brief: &Brief,
    budget: Budget,
) -> Result<Designing, DesignError> {
    let estimate = price::estimate(&brief.passage)?;

    // The disk first, always: three samples sitting there are the answer to
    // "has this been paid for", whatever anybody expected.
    if let Some(session) = session::read(root, &brief.digest()) {
        return Ok(Designing {
            session,
            estimate,
            already_paid: true,
        });
    }

    budget.check(estimate.cents)?;
    // A ledger that will not take the line is refused before paying, not after.
    spending::readable(root)?;
    // Read before this design's record is written, or the seed would count it
    // beside its own line.
    let before = ledger::before_the_ledger(root);
    let candidates = studio.candidates(brief)?;
    if candidates.is_empty() {
        return Err(DesignError::NoCandidates {
            provider: studio.name(),
        });
    }
    let session = session::write(root, brief, &candidates, estimate)?;
    spending::designed(root, &session.digest, estimate.cents, &before)?;
    Ok(Designing {
        session,
        estimate,
        already_paid: false,
    })
}

/// A voice that now exists, and the record that can rebuild it.
#[derive(Debug, Clone, PartialEq)]
pub struct Kept {
    /// The voice, as any listing would describe it.
    pub voice: Voice,
    /// What went into the ledger beside it.
    pub designed: Designed,
}

/// Turns one candidate into a voice in the account.
///
/// **Spends nothing** — the candidate was paid for when it was designed — and
/// is the one call in scorsese with an effect outside the project directory.
/// That is the whole reason the ledger is written here: what this creates does
/// not travel with the project, so what *made* it has to.
///
/// The candidates that were passed over go with the request, which is free and
/// is what the vendor asks for; the id has to come from a design this project
/// actually made, because that record is where the description and the seed
/// are.
pub fn keep(
    root: &Path,
    studio: &dyn Studio,
    chosen: &str,
    name: &str,
) -> Result<Kept, DesignError> {
    let session = session::find(root, chosen).ok_or_else(|| DesignError::NoSuchCandidate {
        chosen: chosen.to_owned(),
    })?;
    let brief = Brief {
        prompt: session.prompt.clone(),
        passage: session.passage.clone(),
        seed: session.seed,
        guidance: session.guidance,
    };

    let voice = studio.keep(&brief, chosen, name, &session.passed_over(chosen))?;
    let designed = Designed {
        voice_id: voice.id.clone(),
        name: voice.name.clone(),
        prompt: session.prompt,
        seed: session.seed,
        guidance: session.guidance,
        passage: session.passage,
        provider: studio.name().to_owned(),
        design: session.digest,
        estimated_cost_cents: session.estimated_cost_cents,
        created_at: Timestamp::now(),
        forgotten: false,
    };
    ledger::append(root, &designed)?;
    Ok(Kept { voice, designed })
}

/// What every surface says after [`forget`], so the command line and the MCP
/// tool cannot drift into telling it differently.
pub const FORGOTTEN: &str = "That voice is gone from the ElevenLabs account, and the voice slot \
     it took is free. A line already generated with it is a file in the project and still \
     plays; a line not yet generated that names it will be refused, so give it another voice. \
     The design and the description and seed that made the voice stay in the project: keeping \
     the same candidate again brings it back while ElevenLabs still holds the candidate, and \
     designing the same description again with the same seed comes nearest after that.";

/// Deletes the voice `voice_id`, kept from one of this project's designs,
/// from the account, and marks it forgotten in the ledger (#1381).
///
/// **The one verb that frees a voice slot.** An account holds a fixed number
/// of designed voices, so one nobody narrates with any more is worth giving
/// back. Spends nothing, and loses nothing that was paid for: the design and
/// its samples stay in `generated/` and the ledger keeps the description and
/// the seed, so keeping the same candidate again brings it back while the
/// vendor still holds the candidate, and designing the same brief again comes
/// nearest after.
///
/// The id has to be one this project keeps — a destructive call is never a
/// guess at which voice was meant, and never reaches a voice somebody else
/// made in the same account.
pub fn forget(root: &Path, studio: &dyn Studio, voice_id: &str) -> Result<Designed, DesignError> {
    let entry = ledger::read(root)
        .into_iter()
        .find(|entry| entry.voice_id == voice_id && entry.kept())
        .ok_or_else(|| DesignError::NotKept {
            voice_id: voice_id.to_owned(),
        })?;
    studio.forget(voice_id)?;
    ledger::forget(root, voice_id)?;
    Ok(Designed {
        forgotten: true,
        ..entry
    })
}
