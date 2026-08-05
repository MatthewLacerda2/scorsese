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
//! written the instant they arrive, before anything can go wrong with them; the
//! ledger entry is written the instant the voice exists. Both are fatal if they
//! fail, because both are the record of money already spent — and the failure
//! mode of being relaxed about it is paying twice for the same three samples,
//! or holding a voice id whose description nobody can recover.
//!
//! **What a repeat costs.** Nothing. Asking for the same design twice is a
//! cache hit; keeping the same candidate twice creates a second voice at the
//! vendor, which is a real effect and so is reported rather than deduplicated —
//! two voices from one candidate is somebody's decision, not an accident this
//! layer should quietly undo.

use std::path::Path;

use scorsese_core::Timestamp;

use crate::credentials::Budget;

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
    let estimate = price::estimate(&brief.passage);

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
    let candidates = studio.candidates(brief)?;
    if candidates.is_empty() {
        return Err(DesignError::NoCandidates {
            provider: studio.name(),
        });
    }
    Ok(Designing {
        session: session::write(root, brief, &candidates, estimate)?,
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
    };
    ledger::append(root, &designed)?;
    Ok(Kept { voice, designed })
}
