//! Every generation a project actually sent, kept beside `project.json` (#1059).
//!
//! # Why the assets are not enough
//!
//! An asset's `estimated_cost_cents` is what **its current file** cost. A shot
//! that came back wrong and was bought again overwrites that figure with the
//! second take's, and the first take — still on disk in `generated/`, still
//! paid for — leaves no trace in the document. A ceiling summed over the
//! assets therefore forgets every retry, and retries are exactly what happens
//! when a generation comes back wrong: the case a ceiling exists for.
//!
//! So each prompted run writes one line here **the moment a brief is sent** —
//! a shot submitted, a still drawn or ordered in a batch, a line spoken, a
//! track composed — and [`so_far`](super::so_far) reads generation spending
//! from these lines. At submit rather than at collection, because a shot in
//! flight has been paid for whether or not it ever arrives (#1107).
//!
//! # Why a file beside `project.json`
//!
//! For the reasons `designed-voices.json` is one
//! ([`crate::voices::design`]): a record of what was spent is not a property of
//! the cut, a field in the document would be a format change, and an
//! append-only list merges by hand in seconds. A file of its own rather than
//! lines inside that one, because that file's name and every surface that
//! lists it say *designed voices*, and a shot is not one.
//!
//! # Projects older than the ledger
//!
//! A project with no lines here is read as one line per asset that carries a
//! figure — today's takes, which is all the document remembers — and the first
//! generation sent writes those lines down, marked `seeded`, before its own.
//! History before the ledger is therefore the current takes only: a take
//! superseded before this file existed is gone from every record.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use scorsese_core::{AssetId, Project, ProjectPath, Timestamp};

use crate::generated::WriteError;

/// What the file is called, beside `project.json`.
pub const GENERATIONS_FILE: &str = "generations.json";

/// One generation that was sent, and what it was worked out to cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sent {
    /// The asset it was for.
    pub asset: AssetId,
    /// The digest of the brief that was sent — the half of the file name in
    /// `generated/` after the asset id, so a take on disk can be matched to the
    /// line that bought it. Empty on a seeded line whose file name says none.
    pub brief: String,
    /// What we worked out it cost. **Our arithmetic, never a bill.**
    pub estimated_cost_cents: u64,
    /// When it was sent. Absent on a seeded line: nobody wrote it down.
    #[serde(default)]
    pub sent_at: Option<Timestamp>,
    /// Copied from the asset's figure when the ledger began, rather than
    /// written when the generation was sent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub seeded: bool,
    /// Turned down by the vendor after it was sent, and not charged for: a
    /// shot Veo's filters block (#1109). Kept, so the file still says it was
    /// sent; never counted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub refused: bool,
}

impl Sent {
    /// What this line adds to what was spent.
    pub fn cents(&self) -> u64 {
        if self.refused {
            0
        } else {
            self.estimated_cost_cents
        }
    }
}

/// Where the ledger lives, inside `root`.
fn file(root: &Path) -> PathBuf {
    root.join(GENERATIONS_FILE)
}

/// The lines on disk, oldest first: `Ok(None)` when there is no file, an error
/// when there is one that does not read.
fn load(root: &Path) -> Result<Option<Vec<Sent>>, WriteError> {
    let path = file(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(WriteError { path, source }),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| WriteError {
            path,
            source: std::io::Error::other(error.to_string()),
        })
}

/// Every generation this project has sent, oldest first — or, when it has no
/// ledger yet, its assets' figures as the lines it would begin with.
///
/// An unreadable file reads as no file, the way `designed-voices.json` does:
/// failing to *read* would put an error path on every quote. Writing is where
/// it is refused ([`sent`]), so a damaged ledger is never overwritten.
pub fn history(project: &Project, root: &Path) -> Vec<Sent> {
    match load(root) {
        Ok(Some(lines)) if !lines.is_empty() => lines,
        _ => seed(project),
    }
}

/// Writes down that `brief` was sent for `id`, at `cents` — seeding the ledger
/// from the assets first when this is its first line.
///
/// Called after the provider accepted the brief and after what it answered is
/// safe (the ticket on the asset, the file on disk), so a ledger that cannot be
/// written stops the run without losing anything that was paid for — and
/// before the asset records the new take's figure, so the seed still holds
/// the take it replaces.
pub(crate) fn sent(
    project: &Project,
    root: &Path,
    id: &AssetId,
    brief: &str,
    cents: u64,
) -> Result<(), WriteError> {
    let mut lines = match load(root)? {
        Some(lines) if !lines.is_empty() => lines,
        _ => seed(project),
    };
    lines.push(Sent {
        asset: id.clone(),
        brief: brief.to_owned(),
        estimated_cost_cents: cents,
        sent_at: Timestamp::now(),
        seeded: false,
        refused: false,
    });
    write(root, &lines)
}

/// Marks the newest line sent for `id` refused — nothing written when there is
/// none, as for a shot sent before the ledger began.
pub(crate) fn refused(root: &Path, id: &AssetId) -> Result<(), WriteError> {
    let Some(mut lines) = load(root)? else {
        return Ok(());
    };
    let Some(line) = lines
        .iter_mut()
        .rev()
        .find(|line| &line.asset == id && !line.seeded && !line.refused)
    else {
        return Ok(());
    };
    line.refused = true;
    write(root, &lines)
}

/// The ledger, as `lines`.
fn write(root: &Path, lines: &[Sent]) -> Result<(), WriteError> {
    let path = file(root);
    let text = serde_json::to_string_pretty(lines).map_err(|error| WriteError {
        path: path.clone(),
        source: std::io::Error::other(error.to_string()),
    })?;
    scorsese_core::write::atomically(&path, text.as_bytes())
        .map_err(|source| WriteError { path, source })
}

/// One seeded line per asset that carries a figure.
fn seed(project: &Project) -> Vec<Sent> {
    project
        .assets
        .iter()
        .filter_map(|asset| {
            Some(Sent {
                asset: asset.id.clone(),
                brief: asset
                    .path
                    .as_ref()
                    .and_then(|path| take(&asset.id, path))
                    .unwrap_or_default()
                    .to_owned(),
                estimated_cost_cents: asset.estimated_cost_cents?,
                sent_at: None,
                seeded: true,
                refused: false,
            })
        })
        .collect()
}

/// The brief digest a take of `id` at `path` was named for — the file name
/// after `<id>-`, without its extension.
pub(crate) fn take<'a>(id: &AssetId, path: &'a ProjectPath) -> Option<&'a str> {
    let name = path.as_str().rsplit('/').next()?;
    let stem = name.split_once('.').map_or(name, |(stem, _)| stem);
    stem.strip_prefix(id.as_str())?.strip_prefix('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_take_is_named_for_the_digest_after_its_asset_id() {
        let id = AssetId::new("s5-gang");
        let take = |path: &str| take(&id, &ProjectPath::new(path)).map(str::to_owned);
        assert_eq!(take("generated/s5-gang-ab12.mp4"), Some("ab12".into()));
        assert_eq!(take("generated/s5-gang-ab12"), Some("ab12".into()));
        assert_eq!(take("generated/s5-gangster-ab12.mp4"), None);
        assert_eq!(take("assets/other.mp4"), None);
    }
}
