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
//! # Designs are lines here too (#1486)
//!
//! A voice design is paid for the moment its candidates come back, whether or
//! not a voice is kept, so it writes its own line here then ([`designed`]) and
//! design spending is read from those lines ([`designs`]). Not from the
//! design's record in `generated/`: that record is overwritten when the same
//! design is paid for again after its samples went, and `generated/` can be
//! deleted or left behind by a copy, while this file sits beside
//! `project.json`. A design line names the design's digest instead of an
//! asset, because a designed voice is not an asset.
//!
//! A project whose ledger holds no design line yet is read the old way — each
//! record in `generated/voice-design/`, merged with the kept voices — and the
//! first design paid writes those down, marked `seeded`, before its own.
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

/// One voice design that was paid for: three candidates, billed once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Design {
    /// The digest of the design's brief — its directory under
    /// `generated/voice-design/`.
    design: String,
    /// What we worked out it cost. **Our arithmetic, never a bill.**
    estimated_cost_cents: u64,
    /// When it was paid for. Absent on a seeded line.
    #[serde(default)]
    sent_at: Option<Timestamp>,
    /// Copied from the design records when design lines began, rather than
    /// written when the design was paid for.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    seeded: bool,
}

/// A line of the file: a generation, or a design. Told apart by their fields —
/// a generation names an `asset`, a design a `design` — so the lines written
/// before designs were here read unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Line {
    Sent(Sent),
    Design(Design),
}

impl Line {
    fn sent(&self) -> Option<&Sent> {
        match self {
            Self::Sent(sent) => Some(sent),
            Self::Design(_) => None,
        }
    }

    fn design(&self) -> Option<&Design> {
        match self {
            Self::Design(design) => Some(design),
            Self::Sent(_) => None,
        }
    }
}

/// Where the ledger lives, inside `root`.
fn file(root: &Path) -> PathBuf {
    root.join(GENERATIONS_FILE)
}

/// The lines on disk, oldest first: `Ok(None)` when there is no file, an error
/// when there is one that does not read.
fn load(root: &Path) -> Result<Option<Vec<Line>>, WriteError> {
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
/// failing to *read* would put an error path on every quote. Sending is where
/// it is refused, before anything is paid for, and so is
/// writing, so a damaged ledger is never overwritten.
pub fn history(project: &Project, root: &Path) -> Vec<Sent> {
    let sent: Vec<Sent> = load(root)
        .ok()
        .flatten()
        .unwrap_or_default()
        .iter()
        .filter_map(Line::sent)
        .cloned()
        .collect();
    if sent.is_empty() { seed(project) } else { sent }
}

/// What every design line says was spent designing voices — `None` while the
/// ledger holds none, so the caller reads the projects older than them the old
/// way. Each line is one payment: a design paid for twice counts twice.
pub(crate) fn designs(root: &Path) -> Option<u64> {
    let lines = load(root).ok().flatten()?;
    let mut designs = lines.iter().filter_map(Line::design).peekable();
    designs.peek()?;
    Some(designs.map(|design| design.estimated_cost_cents).sum())
}

/// Writes down that the design `digest` was paid for, at `cents` — seeding the
/// design lines from `before` (`(digest, cents)` for every design paid before
/// they began) when this is the first.
///
/// Called once the samples and their record are on disk, so a ledger that
/// cannot be written stops the run without losing what was paid for;
/// [`readable`] is asked before the vendor is.
pub(crate) fn designed(
    root: &Path,
    digest: &str,
    cents: u64,
    before: &[(String, u64)],
) -> Result<(), WriteError> {
    let mut lines = load(root)?.unwrap_or_default();
    if !lines.iter().any(|line| line.design().is_some()) {
        lines.extend(before.iter().map(|(design, cents)| {
            Line::Design(Design {
                design: design.clone(),
                estimated_cost_cents: *cents,
                sent_at: None,
                seeded: true,
            })
        }));
    }
    lines.push(Line::Design(Design {
        design: digest.to_owned(),
        estimated_cost_cents: cents,
        sent_at: Timestamp::now(),
        seeded: false,
    }));
    write(root, &lines)
}

/// Refuses a ledger that is there and does not read — called before a brief
/// is sent, so a run never pays first and then stops on a line it cannot
/// write, leaving a file the next run takes for cached and never counts.
pub(crate) fn readable(root: &Path) -> Result<(), WriteError> {
    load(root).map(drop)
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
    sent_all(project, root, &[(id.clone(), brief.to_owned(), cents)])
}

/// [`sent`] for every `(asset, brief, cents)` at once, in one write — a batch
/// job, whose tickets are all on their assets before any line is written.
pub(crate) fn sent_all(
    project: &Project,
    root: &Path,
    takes: &[(AssetId, String, u64)],
) -> Result<(), WriteError> {
    let mut lines = load(root)?.unwrap_or_default();
    if !lines.iter().any(|line| line.sent().is_some()) {
        lines.extend(seed(project).into_iter().map(Line::Sent));
    }
    let sent_at = Timestamp::now();
    lines.extend(takes.iter().map(|(id, brief, cents)| {
        Line::Sent(Sent {
            asset: id.clone(),
            brief: brief.clone(),
            estimated_cost_cents: *cents,
            sent_at: sent_at.clone(),
            seeded: false,
            refused: false,
        })
    }));
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
        .filter_map(|line| match line {
            Line::Sent(sent) => Some(sent),
            Line::Design(_) => None,
        })
        .find(|line| &line.asset == id && !line.seeded)
    else {
        return Ok(());
    };
    line.refused = true;
    write(root, &lines)
}

/// Every line sent for an asset `renamed` lists, moved to its new id — what
/// `scorsese migrate` does when a migration renames ids, before it writes the
/// migrated document (`scorsese_core::migrate::folder_then`), so the takes an
/// asset shows are still its own and a refund still finds its line. Nothing
/// is written when no line names one. The error names the file.
pub fn follow_renames(root: &Path, renamed: &[(AssetId, AssetId)]) -> std::io::Result<()> {
    let said = |WriteError { path, source }| {
        std::io::Error::new(source.kind(), format!("{}: {source}", path.display()))
    };
    let Some(mut lines) = load(root).map_err(said)? else {
        return Ok(());
    };
    let mut moved = false;
    for line in &mut lines {
        let Line::Sent(sent) = line else { continue };
        if let Some((_, new)) = renamed.iter().find(|(old, _)| *old == sent.asset) {
            sent.asset = new.clone();
            moved = true;
        }
    }
    if moved {
        write(root, &lines).map_err(said)
    } else {
        Ok(())
    }
}

/// The ledger, as `lines`.
fn write(root: &Path, lines: &[Line]) -> Result<(), WriteError> {
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
