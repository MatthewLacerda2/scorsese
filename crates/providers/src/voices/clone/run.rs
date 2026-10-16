//! Cloning a voice, and forgetting one — the two verbs, and what each asks
//! before anything reaches the vendor.
//!
//! **Consent is asked first.** Before a name is read or a file opened, the
//! caller says consent was given for this voice ([`check`]): the vendor's API
//! takes no such field, so scorsese asks where the vendor's own app does.
//!
//! **What gets written down, and when.** The ledger entry the instant the
//! voice exists, fatal if it fails, as a designed voice's is: the voice takes
//! a slot in the account, and an unrecorded one is a slot nobody accounts for.

use std::path::Path;

use scorsese_core::{AssetId, Project, Timestamp};

use super::super::Voice;
use super::super::design::Studio;
use super::Request;
use super::error::CloneError;
use super::ledger::{self, Cloned};
use super::recordings::{advice, gather};

/// What a surface asks to clone, from a project's own recordings.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    /// The audio assets to clone from.
    pub assets: Vec<AssetId>,
    /// What to call the voice.
    pub name: String,
    /// What the voice is, kept with it at the vendor.
    pub description: Option<String>,
    /// Ask the vendor to clean background noise out first.
    pub remove_background_noise: bool,
    /// Whether consent was given: the voice is the caller's own, or its owner
    /// has given permission.
    pub consent: bool,
}

/// What a studio answers a clone with.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// The voice, as any listing would describe it.
    pub voice: Voice,
    /// Whether the vendor wants it verified before it can be used.
    pub requires_verification: bool,
}

/// A clone that worked: the voice, its record, and the vendor's advice on the
/// recordings where they were outside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    /// The voice.
    pub voice: Voice,
    /// What went into the ledger.
    pub cloned: Cloned,
    /// The vendor's advice on how much to record, when it applies.
    pub advice: Option<String>,
}

/// Refused unless consent was given and the voice has a name — in that order,
/// so the consent sentence is the first thing anybody reads.
pub fn check(consent: bool, name: &str) -> Result<(), CloneError> {
    if !consent {
        return Err(CloneError::NoConsent);
    }
    if name.trim().is_empty() {
        return Err(CloneError::NoName);
    }
    Ok(())
}

/// Clones a voice from `ask`'s recordings in `project` at `root`, and writes
/// down what made it.
pub fn clone(
    root: &Path,
    project: &Project,
    studio: &dyn Studio,
    ask: &Ask,
) -> Result<Done, CloneError> {
    check(ask.consent, &ask.name)?;
    let gathered = gather(root, project, &ask.assets)?;
    let made = studio.clone_voice(&Request {
        name: ask.name.trim().to_owned(),
        description: ask.description.clone(),
        recordings: gathered.recordings,
        remove_background_noise: ask.remove_background_noise,
    })?;
    let cloned = Cloned {
        voice_id: made.voice.id.clone(),
        name: made.voice.name.clone(),
        description: ask.description.clone(),
        recordings: gathered.sources,
        remove_background_noise: ask.remove_background_noise,
        provider: studio.name().to_owned(),
        created_at: Timestamp::now(),
        requires_verification: made.requires_verification,
        forgotten: false,
    };
    ledger::append(root, &cloned)?;
    Ok(Done {
        voice: made.voice,
        cloned,
        advice: advice(gathered.seconds),
    })
}

/// What every surface says after a clone is forgotten.
pub const FORGOTTEN: &str = "That voice is gone from the ElevenLabs account, and the voice slot \
     it took is free. A line already generated with it is a file in the project and still \
     plays; a line not yet generated that names it will be refused, so give it another voice. \
     The recordings it was cloned from are still assets of the project, so cloning them again \
     brings a voice like it back.";

/// Deletes the voice `voice_id`, cloned in this project, from the account, and
/// marks it forgotten in the ledger. The id has to be one this project's
/// ledger keeps: a deletion is never a guess.
pub fn forget(root: &Path, studio: &dyn Studio, voice_id: &str) -> Result<Cloned, CloneError> {
    let entry = ledger::read(root)
        .into_iter()
        .find(|entry| entry.voice_id == voice_id && entry.kept())
        .ok_or_else(|| CloneError::NotKept {
            voice_id: voice_id.to_owned(),
        })?;
    studio.forget(voice_id)?;
    ledger::forget(root, voice_id)?;
    Ok(Cloned {
        forgotten: true,
        ..entry
    })
}
