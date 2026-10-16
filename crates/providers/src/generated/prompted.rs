//! What a run does the same way for a shot, a still and a line — finds the
//! kind's assets, takes a file already on disk, adopts what a caller laid out,
//! and reads what a run would do — over a [`Prompted`] that names what is each
//! kind's own (#1279).
//!
//! These used to be written three times, once per provider, and had begun to
//! drift. What stays each kind's own is so on purpose:
//!
//! - **its `Outcome`**, because the variants *are* the difference between the
//!   kinds: only a shot can be waited on or expire, only a still can be ordered
//!   in a batch, and a line can be neither. Each also has its own sentences;
//! - **the step that spends**: a shot submits and polls a ticket, a still is
//!   drawn, a line is spoken with its word timings;
//! - **speech's [`Plan`](crate::speech::Plan)**, which has no in-flight
//!   variant, since nothing is ever in flight there.

use std::fmt::Display;
use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};

/// One prompted kind of asset, as its run sees it.
pub(crate) trait Prompted {
    /// The kind of asset this generates.
    const KIND: AssetKind;
    /// What a generation of one asset is asked from.
    type Brief;
    /// Why a brief cannot be gathered yet.
    type Unready: Display;
    /// What stops a run.
    type Error;

    /// Whether `asset` is one this run generates: every asset of
    /// [`KIND`](Self::KIND), unless the kind is shared — `generated_audio` is
    /// a line or a track by the block it carries (#1264), and each run claims
    /// its own.
    fn claims(asset: &Asset) -> bool {
        asset.kind == Self::KIND
    }

    /// The asset's brief, or why it is not ready.
    fn brief(project: &Project, root: &Path, asset: &Asset) -> Result<Self::Brief, Self::Unready>;

    /// Where a generation of `brief` lands, project-relative.
    fn output(brief: &Self::Brief) -> ProjectPath;

    /// Whether a generation of `brief` is already on disk.
    fn realized(brief: &Self::Brief, root: &Path) -> bool;

    /// Records `bytes`, at `path`, as the generation of `id`'s brief —
    /// through [`record`](super::record), priced the kind's way.
    fn keep(
        project: &mut Project,
        root: &Path,
        id: &AssetId,
        path: &ProjectPath,
        bytes: &[u8],
        brief: &Self::Brief,
    ) -> Result<(), Self::Error>;
}

/// Every asset the kind [claims](Prompted::claims), by id, in document order.
pub(crate) fn ids<K: Prompted>(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| K::claims(asset))
        .map(|asset| asset.id.clone())
        .collect()
}

/// Records the file `brief` already has on disk against `id` — a cache hit —
/// and says where it is.
pub(crate) fn found<K: Prompted>(
    project: &mut Project,
    root: &Path,
    id: &AssetId,
    brief: &K::Brief,
) -> Result<ProjectPath, K::Error> {
    let output = K::output(brief);
    let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
    K::keep(project, root, id, &output, &bytes, brief)?;
    Ok(output)
}

/// Points every asset of the kind whose current brief's output is already on
/// disk at that file, and says which it found — what each kind's `adopt` is.
pub(crate) fn adopt<K: Prompted>(project: &mut Project, root: &Path) -> Vec<AssetId> {
    let mut adopted = Vec::new();
    for id in ids::<K>(project) {
        let Some(brief) = project
            .asset(&id)
            .and_then(|asset| K::brief(project, root, asset).ok())
            .filter(|brief| K::realized(brief, root))
        else {
            continue;
        };
        if found::<K>(project, root, &id, &brief).is_ok() {
            adopted.push(id);
        }
    }
    adopted
}

/// What a run would do with one shot or still, read without doing it.
///
/// The pass's own decision order — the cache first, then the ticket, then a
/// submission — answered before any key is resolved and without spending
/// anything. It exists for the callers that come *before* a run: the
/// [quote](crate::quote), and the check that decides whether a pass is worth
/// resolving a key for. Both used to consult the asset's recorded `path`
/// instead, and that is the file the *previous* generation landed in — after
/// an edit it still exists and still resolves, which is exactly how a stale
/// shot got skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The current brief's output is already in `generated/`. Nothing would be
    /// sent and nothing paid.
    Realized(
        /// Where it is, project-relative.
        ProjectPath,
    ),
    /// A ticket is in flight — a shot generating, or a still waiting in a
    /// batch — already paid for. A run polls or collects it and spends nothing
    /// new.
    InFlight,
    /// The brief would be handed to the provider and billed.
    Submit,
    /// The brief cannot be gathered, in the words a run would refuse it with.
    Unready(
        /// What is wrong with it.
        String,
    ),
}

/// What a run would do with `asset` — see [`Plan`].
pub(crate) fn plan<K: Prompted>(project: &Project, root: &Path, asset: &Asset) -> Plan {
    match K::brief(project, root, asset) {
        Err(why) => Plan::Unready(why.to_string()),
        Ok(brief) if K::realized(&brief, root) => Plan::Realized(K::output(&brief)),
        Ok(_) if asset.operation.is_some() => Plan::InFlight,
        Ok(_) => Plan::Submit,
    }
}

/// Whether a run over the kind's assets would have anything to do at all.
///
/// A dangling ticket counts even when the cache would answer it — the run
/// still clears it out of the document. An ungatherable brief counts too,
/// because the pass is where that refusal is voiced, naming what is missing.
pub(crate) fn pending<K: Prompted>(project: &Project, root: &Path) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| K::claims(asset))
        .any(|asset| {
            asset.operation.is_some()
                || !matches!(plan::<K>(project, root, asset), Plan::Realized(_))
        })
}
