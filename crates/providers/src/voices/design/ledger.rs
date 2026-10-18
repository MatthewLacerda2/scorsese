//! What made each designed voice, kept in the project.
//!
//! # The problem this file exists for
//!
//! Everything else scorsese creates lands inside the project directory, which
//! is what makes the promise that a project survives `scp -r` a real one.
//! **A designed voice does not.** It is created in somebody's ElevenLabs
//! account, and all the project can hold is its id — so the id travels and the
//! voice does not. Open the project on another machine, under another account,
//! or after somebody tidied their voice list, and the id names nothing.
//!
//! Recording only the id would make that loss unrecoverable. Recording **the
//! description and the seed** makes it a setback: the same sentence and the
//! same seed, designed again, is the nearest thing there is to asking for the
//! same voice twice. The vendor calls the seed best-effort and it is written
//! down as exactly that — but a best-effort attempt at the voice somebody chose
//! beats a dead id and no idea what it sounded like.
//!
//! # Why this is a file beside `project.json` rather than a field inside it
//!
//! Three reasons, in the order they decided it.
//!
//! **It is not the document's business.** `project.json` describes a cut —
//! assets, tracks, clips. A record of things created in an account elsewhere is
//! a different subject, and the format is a contract shared by the CLI, the MCP
//! server, the window and every saved project.
//!
//! **A schema change is `architecture` work**, needing a version bump and a
//! migration note. The `speech` block does now hold a `voice_id`, so a home
//! could be found — but what would go there is not a property of the cut. An
//! asset says *which* voice reads a line; this says how a voice that lives in
//! somebody's account came to exist, which is true of the account and not of
//! the video. A project copied to a machine signed in as someone else keeps
//! every word of its `speech` blocks and none of these voices.
//!
//! **It is append-only and merge-friendly.** One entry per design, never
//! removed — a forgotten voice is only marked so (#1381) — so two sessions
//! designing voices produce a file that merges by hand in seconds. A growing array inside `project.json` would put that churn
//! in the middle of the document a person is editing.
//!
//! It is deliberately **not** under `cache/`: cache is gitignored and
//! rebuildable, and this is neither. Losing it is the exact failure the file
//! exists to prevent.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use scorsese_core::Timestamp;

use super::error::DesignError;

/// What the file is called, beside `project.json`.
pub const LEDGER_FILE: &str = "designed-voices.json";

/// One voice that was designed, and everything needed to attempt it again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Designed {
    /// The id a narration names. What travels, and the only part of this that
    /// the vendor knows about.
    pub voice_id: String,
    /// What it was called when it was created.
    pub name: String,
    /// The description it was designed from. **The half that makes a lost
    /// voice recoverable.**
    pub prompt: String,
    /// The determinism knob it was designed with, where there was one. The
    /// other half.
    #[serde(default)]
    pub seed: Option<u32>,
    /// How literally the description was followed, where that was set.
    #[serde(default)]
    pub guidance: Option<f64>,
    /// The passage the candidates read. Recorded because it is what was billed,
    /// and because designing again with a different passage is designing
    /// something else.
    pub passage: String,
    /// Who it was designed at, so an id that names nothing can be asked about
    /// in the right place.
    pub provider: String,
    /// The fingerprint of the design this came out of.
    ///
    /// Here so that [`spent`] can tell one design from two. Keeping a second
    /// candidate out of the same three costs nothing — they were all paid for
    /// by one call — and a total that added the design's price again for each
    /// voice kept from it would over-count a spend nobody made.
    pub design: String,
    /// What we worked out the design cost. **Our arithmetic, never a bill.**
    pub estimated_cost_cents: u64,
    /// When it was created, as a person reads a date.
    #[serde(default)]
    pub created_at: Option<Timestamp>,
    /// Whether it was deleted from the account again (#1381), freeing the
    /// voice slot it took. Left out of the file while the voice is kept.
    ///
    /// Marked rather than removed: the entry is still what the design cost,
    /// which a spending ceiling counts ([`spent`]), and still the description
    /// and the seed — keeping the same candidate again is how a forgotten
    /// voice comes back.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub forgotten: bool,
}

impl Designed {
    /// The one line a listing prints for this voice.
    ///
    /// Assembled here rather than at each surface, for the same reason
    /// [`Voice::says`](super::super::Voice::says) is: the command line and the
    /// MCP tool must not drift into describing the same voice differently.
    pub fn says(&self) -> String {
        let seed = self
            .seed
            .map_or_else(|| String::from("no seed"), |seed| format!("seed {seed}"));
        let forgotten = if self.forgotten { ", forgotten" } else { "" };
        format!(
            "{}  {}  ({seed}{forgotten})\n    {}",
            self.voice_id, self.name, self.prompt
        )
    }

    /// Whether the voice is still in the account, as far as this project
    /// knows.
    pub fn kept(&self) -> bool {
        !self.forgotten
    }
}

/// Where the ledger lives, inside `root`.
fn file(root: &Path) -> PathBuf {
    root.join(LEDGER_FILE)
}

/// Every voice this project has designed, oldest first.
///
/// An unreadable or absent file is an empty ledger rather than a failure. A
/// project that has never designed a voice has no file at all, and that is the
/// overwhelmingly common case — failing to *read* here would make every other
/// surface carry an error path for "you have not used this feature".
pub fn read(root: &Path) -> Vec<Designed> {
    std::fs::read_to_string(file(root))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// What this project has spent designing voices.
///
/// **Its own figure, and it stays its own.** Folding it into what the shots
/// cost would produce a narration total that moves while nobody is generating
/// narration, and a counter that does that is a counter nobody trusts. It is
/// still real money, so a spending ceiling counts it — being a separate line
/// item is about how it is *reported*, not about whether it is spent.
///
/// Counted **once per design**, not once per voice. One design call answers
/// with three candidates and is billed once, so somebody who kept two of them
/// spent what one design costs — see [`Designed::design`].
///
/// And counted **whether or not a voice was kept, once per payment** (#1107,
/// #1486): [`design`] writes a line into `generations.json` the moment the
/// candidates come back, and this reads those lines — so a design paid for
/// again after its samples went counts twice, and deleting `generated/`
/// refunds nothing.
///
/// A project with no design line yet is read the way it was before them: each
/// record in `generated/voice-design/`, and each kept voice whose record has
/// gone, once per digest ([`before_the_ledger`]).
///
/// [`design`]: super::design
pub fn spent(root: &Path) -> u64 {
    crate::spending::designs(root)
        .unwrap_or_else(|| before_the_ledger(root).iter().map(|(_, cents)| cents).sum())
}

/// `(digest, cents)` for every design this project paid for that the records
/// on disk still show — each record in `generated/voice-design/`, merged by
/// digest with the kept voices here. What the first design line seeds the
/// ledger with.
pub(super) fn before_the_ledger(root: &Path) -> Vec<(String, u64)> {
    let mut designs = super::session::paid(root);
    for designed in read(root) {
        designs.push((designed.design, designed.estimated_cost_cents));
    }
    let mut counted: Vec<(String, u64)> = Vec::new();
    for (digest, cents) in designs {
        if !counted.iter().any(|(seen, _)| *seen == digest) {
            counted.push((digest, cents));
        }
    }
    counted
}

/// Adds a voice to the ledger, creating the file if this is the project's
/// first.
///
/// **Read, append, write** rather than an append to the end of a JSON array,
/// because the file has to stay a document a person can open. It is a handful
/// of entries at most, so the cost of rewriting it is nothing next to the cost
/// of it being unreadable.
///
/// Failing here is fatal, and that is deliberate: the voice already exists at
/// the vendor and has already been paid for, so a ledger that quietly did not
/// save is the exact unrecoverable loss this file exists to prevent. The caller
/// reports the id in the same breath, so nothing is lost that a person cannot
/// write down.
pub(super) fn append(root: &Path, designed: &Designed) -> Result<(), DesignError> {
    let mut entries = read(root);
    entries.push(designed.clone());
    write(root, &entries)
}

/// Marks the kept voice `voice_id` forgotten — `false`, and nothing written,
/// when no entry keeps it.
pub(super) fn forget(root: &Path, voice_id: &str) -> Result<bool, DesignError> {
    let mut entries = read(root);
    let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.voice_id == voice_id && entry.kept())
    else {
        return Ok(false);
    };
    entry.forgotten = true;
    write(root, &entries).map(|()| true)
}

/// The ledger, as `entries`.
fn write(root: &Path, entries: &[Designed]) -> Result<(), DesignError> {
    let text = serde_json::to_string_pretty(entries).map_err(|error| DesignError::Write {
        doing: String::from("describing the designed voice"),
        source: std::io::Error::other(error.to_string()),
    })?;
    let path = file(root);
    scorsese_core::write::atomically(&path, text.as_bytes()).map_err(|source| DesignError::Write {
        doing: format!("writing {}", path.display()),
        source,
    })
}
