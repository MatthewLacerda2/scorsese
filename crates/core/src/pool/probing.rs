//! Filling in the metadata nobody has read yet.
//!
//! Importing probes. For a long time nothing else did — so an asset that
//! reached `project.json` any other way (written by hand, written by an agent
//! assembling a cut, added by a tool that is not `scorsese import`) carried a
//! path and no `media`, and stayed that way for the life of the project.
//!
//! That was harmless while nothing depended on the metadata. It stops being
//! harmless the moment something does. `media.duration_seconds` is the only
//! record of how long a source actually is, and a feature that wants it has to
//! handle its absence — where the honest handling is always "then do nothing",
//! which means the feature quietly fails to work for whatever fraction of the
//! pool nobody has looked at. A ceiling on trimming is the first thing to need
//! it and it will not be the last: fit, conform and the render plan all read
//! this block. A rule that applies to half the pool is worse than no rule,
//! because the half it skips is invisible to the person it skips it for.
//!
//! So probing is something that can be **asked for** across the whole pool —
//! from the command line, from MCP, and from the window when it opens a
//! project. Like importing, it takes its [`ProbeMedia`] as an argument: core
//! declares that seam and never implements it, because probing means running
//! ffprobe and this crate does not spawn processes.

use std::path::Path;

use crate::asset::{Asset, AssetId, AssetKind, GenerationState, MediaMetadata};
use crate::probe::ProbeMedia;
use crate::project::Project;

/// Whether an asset that already carries metadata is looked at again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reprobe {
    /// Leave a probed asset alone. This is what makes probing idempotent, and
    /// therefore cheap enough to run on every open and after every edit: a
    /// pool that has already been probed costs one pass over a `Vec` rather
    /// than one process per file.
    Skip,
    /// Read every file again, replacing the metadata that is recorded. For
    /// when what is written down is believed to be wrong. Never the default —
    /// re-reading a pool of large sources is real work, and the answer almost
    /// never changes.
    All,
}

/// What probing did about one asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The prober answered, and the answer is now in the assets table.
    Recorded,
    /// Metadata was already there and [`Reprobe::Skip`] left it alone.
    AlreadyKnown,
    /// The assets table points at a file that is not on disk, so there was
    /// nothing to read. Reported rather than passed over silently: while
    /// looking at every file in the pool, noticing a missing one is free.
    Missing,
    /// The prober refused. The asset is left exactly as it was — a probe that
    /// fails never half-fills a `media` block, and the message is passed
    /// through so whoever asked can see what ffprobe actually said.
    Failed(String),
}

/// One asset that probing had something to say about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    /// The asset this is about.
    pub id: AssetId,
    /// What happened to it.
    pub outcome: ProbeOutcome,
}

/// Probes every asset that is supposed to have a file, recording what comes
/// back, and reports what it did about each one.
///
/// Assets with nothing to probe do not appear in the report at all: an inline
/// `text` or `color` asset has no file by construction, and a generated asset
/// that is still a sketch has none *yet*. Neither is an omission anybody can
/// act on, and listing them would bury the ones that are.
///
/// Nothing is written to disk here. The caller holds the document and decides
/// when it is saved, which is what lets the window probe in the background
/// without racing its own writer.
pub fn probe_assets(
    project: &mut Project,
    project_root: &Path,
    probe: &dyn ProbeMedia,
    reprobe: Reprobe,
) -> Vec<Probed> {
    project
        .assets
        .iter_mut()
        .filter(|asset| expects_a_file(asset))
        .map(|asset| Probed {
            id: asset.id.clone(),
            outcome: probe_one(asset, project_root, probe, reprobe),
        })
        .collect()
}

/// The assets a probe would read and that nobody has read yet.
///
/// Answered without touching the filesystem, which is the whole point of it
/// being separate from [`probe_assets`]: the window asks this on every open to
/// decide whether there is any work to hand to a background thread, and for a
/// project assembled through `scorsese import` the answer is none — so no
/// thread starts and no process is spawned.
pub fn unprobed_assets(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| expects_a_file(asset) && asset.media.is_none())
        .map(|asset| asset.id.clone())
        .collect()
}

/// Probes one asset and records what came back on it.
fn probe_one(
    asset: &mut Asset,
    project_root: &Path,
    probe: &dyn ProbeMedia,
    reprobe: Reprobe,
) -> ProbeOutcome {
    if reprobe == Reprobe::Skip && asset.media.is_some() {
        return ProbeOutcome::AlreadyKnown;
    }
    let Some(file) = asset.path.as_ref().map(|path| path.resolve(project_root)) else {
        return ProbeOutcome::Missing;
    };
    if !file.is_file() {
        return ProbeOutcome::Missing;
    }
    match probe.probe(&file) {
        Ok(media) => {
            asset.media = Some(as_recorded(asset.kind, media));
            ProbeOutcome::Recorded
        }
        Err(error) => ProbeOutcome::Failed(error.message),
    }
}

/// True when a file on disk is what this asset ultimately refers to *now*.
///
/// The mirror of what [`super::status::asset_status`] calls `Inline` and
/// `Awaiting`: an inline kind has no file by construction, and a generated
/// asset in any state before `generated` has none yet. Everything else is
/// something a prober could be asked about — including one whose file has
/// gone, which is reported rather than skipped.
fn expects_a_file(asset: &Asset) -> bool {
    asset.kind.is_file_backed() && asset.state.is_none_or(GenerationState::has_media)
}

/// What gets *recorded* for a kind, which is not always what the prober said.
///
/// ffprobe calls a still a one-frame video: it invents a frame rate for it and
/// sometimes a duration, and neither number means anything — how long a still
/// is on screen is the clip's business, not the file's. Import has always
/// dropped both, and probing has to drop them the same way, or one file would
/// be described differently depending on how it entered the project.
///
/// That is more than tidiness. `duration_seconds` is what bounds a trim, so a
/// still recorded as four hundredths of a second long would be a still nobody
/// could hold on screen.
pub(super) fn as_recorded(kind: AssetKind, media: MediaMetadata) -> MediaMetadata {
    if kind == AssetKind::Image {
        return MediaMetadata {
            frame_rate: None,
            duration_seconds: None,
            ..media
        };
    }
    media
}
