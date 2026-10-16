//! What made each cloned voice, kept in the project: `cloned-voices.json`.
//!
//! The sibling of `designed-voices.json` (`design::ledger` has the long
//! reason for a file beside `project.json` rather than a field in it), and
//! for the same problem: the voice lives in somebody's ElevenLabs account,
//! and the project can only carry its id. What makes a cloned voice
//! recoverable is not a seed but **the recordings**, which are assets of the
//! project already — so each entry names them, by asset id, path and hash,
//! and cloning them again is how a lost voice comes back.
//!
//! Append-only, a forgotten voice only marked so, exactly as the design
//! ledger is.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use scorsese_core::{AssetId, ProjectPath, Timestamp};

use super::error::CloneError;

/// What the file is called, beside `project.json`.
pub const LEDGER_FILE: &str = "cloned-voices.json";

/// One recording a voice was cloned from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// The asset it is.
    pub asset: AssetId,
    /// Where its file is, project-relative.
    pub path: ProjectPath,
    /// The file's hash when it was sent, so a recording replaced since is
    /// told from the one that made the voice.
    #[serde(default)]
    pub sha256: Option<String>,
}

/// One voice that was cloned, and what made it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cloned {
    /// The id a narration names.
    pub voice_id: String,
    /// What it was called in the account.
    pub name: String,
    /// What it was described as, where it was.
    #[serde(default)]
    pub description: Option<String>,
    /// The recordings it was cloned from — what makes it recoverable.
    pub recordings: Vec<Source>,
    /// Whether the vendor was asked to clean background noise out first.
    #[serde(default)]
    pub remove_background_noise: bool,
    /// Who it was cloned at.
    pub provider: String,
    /// When it was cloned.
    #[serde(default)]
    pub created_at: Option<Timestamp>,
    /// Whether the vendor said the voice needs verifying before it is used.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_verification: bool,
    /// Whether it was deleted from the account again, freeing its slot.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub forgotten: bool,
}

impl Cloned {
    /// The one line a listing prints for this voice, on every surface.
    pub fn says(&self) -> String {
        let from: Vec<&str> = self
            .recordings
            .iter()
            .map(|source| source.asset.as_str())
            .collect();
        let forgotten = if self.forgotten { ", forgotten" } else { "" };
        format!(
            "{}  {}  (cloned{forgotten})\n    from {}",
            self.voice_id,
            self.name,
            from.join(", ")
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

/// Every voice this project has cloned, oldest first. An absent or
/// unreadable file is an empty ledger, as the design ledger's is.
pub fn read(root: &Path) -> Vec<Cloned> {
    std::fs::read_to_string(file(root))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Adds a voice to the ledger.
pub(super) fn append(root: &Path, cloned: &Cloned) -> Result<(), CloneError> {
    let mut entries = read(root);
    entries.push(cloned.clone());
    write(root, &entries)
}

/// Marks the kept voice `voice_id` forgotten — `false`, and nothing written,
/// when no entry keeps it.
pub(super) fn forget(root: &Path, voice_id: &str) -> Result<bool, CloneError> {
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
fn write(root: &Path, entries: &[Cloned]) -> Result<(), CloneError> {
    let text = serde_json::to_string_pretty(entries).map_err(|error| CloneError::Write {
        doing: String::from("describing the cloned voice"),
        source: std::io::Error::other(error.to_string()),
    })?;
    let path = file(root);
    scorsese_core::write::atomically(&path, text.as_bytes()).map_err(|source| CloneError::Write {
        doing: format!("writing {}", path.display()),
        source,
    })
}
