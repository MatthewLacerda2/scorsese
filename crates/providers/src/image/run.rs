//! The lifecycle: sketch → generated, in one call.
//!
//! [`speech`](crate::speech)'s run, for a picture drawn now: an incomplete
//! brief is that still's outcome rather than the end of the run, and what
//! stops a run is what makes every remaining still impossible. A still waiting
//! in a batch is [`collect`](super::collect)'s, which every run calls first.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, Project, ProjectPath, hash_bytes};

use crate::credentials::Budget;
use crate::prices::{self, ImageEstimate, UnpricedImage};

use super::{Brief, ImageError, ImageProvider, Outcome, collect};

/// Draws every still that needs it, and reports what happened to each.
///
/// **Idempotent by construction, and that is the money guarantee**: a still
/// whose file already exists is recorded and never sent. The project is left
/// describing what is on disk; saving it is the caller's, **even when this
/// returns an error** — a still drawn before the failure has been paid for.
///
/// Whatever is waiting in a batch is collected first, and a still the
/// collection touched is not drawn again on the same run, even one whose batch
/// failed: the quote this run was agreed on priced it at nothing.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, ImageError> {
    let mut done = collect(project, root, provider)?;
    let mut spent = 0;
    for id in still_ids(project) {
        if done.iter().any(|(handled, _)| handled == &id) {
            continue;
        }
        let outcome = one(project, root, provider, budget.spend(spent), &id)?;
        spent += outcome.spent_cents();
        done.push((id, outcome));
    }
    Ok(done)
}

/// Points every still whose current brief's output is already on disk at that
/// file — the cache half of a run, with no provider and nothing spent. Which
/// stills it found. The sibling of [`video::adopt`](crate::video::adopt).
pub fn adopt(project: &mut Project, root: &Path) -> Vec<AssetId> {
    let mut adopted = Vec::new();
    for id in still_ids(project) {
        let Some(brief) = project
            .asset(&id)
            .and_then(|asset| Brief::of(project, root, asset).ok())
            .filter(|brief| brief.realized(root))
        else {
            continue;
        };
        let output = brief.output();
        let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
        if let Ok(priced) = estimate(&brief) {
            record(project, &id, &output, &bytes, priced.cents);
            adopted.push(id);
        }
    }
    adopted
}

/// What a run would do with one still, read without doing it — the sibling of
/// [`speech::Plan`](crate::speech::Plan), for the quote and for whether a pass
/// is worth resolving a key for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The current brief's output is already in `generated/`.
    Realized(
        /// Where it is, project-relative.
        ProjectPath,
    ),
    /// Waiting in a batch, already paid for. A run collects it and spends
    /// nothing new.
    InFlight,
    /// The brief would be handed to the provider and billed.
    Submit,
    /// Not ready to be drawn, in [`Incomplete`](super::Incomplete)'s words.
    Unready(
        /// What it is still missing.
        String,
    ),
}

/// What a run would do with `asset` — see [`Plan`].
pub fn plan(project: &Project, root: &Path, asset: &Asset) -> Plan {
    match Brief::of(project, root, asset) {
        Err(why) => Plan::Unready(why.to_string()),
        Ok(brief) if brief.realized(root) => Plan::Realized(brief.output()),
        Ok(_) if asset.operation.is_some() => Plan::InFlight,
        Ok(_) => Plan::Submit,
    }
}

/// Whether a run over this project's stills would have anything to do — an
/// incomplete one counts, because the pass is where *not yet* is voiced, and
/// so does any ticket, which the run collects or clears.
pub fn pending(project: &Project, root: &Path) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedImage)
        .any(|asset| {
            asset.operation.is_some() || !matches!(plan(project, root, asset), Plan::Realized(_))
        })
}

/// What drawing `brief` is expected to cost — the one calculation the quote,
/// the ceiling and the record all use.
pub(crate) fn estimate(brief: &Brief) -> Result<ImageEstimate, UnpricedImage> {
    prices::image(
        &brief.request,
        brief.characters(),
        brief.reference_images.len(),
    )
}

/// Every `generated_image` asset, by id.
pub(super) fn still_ids(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedImage)
        .map(|asset| asset.id.clone())
        .collect()
}

/// Draws one still, or says why it was not drawn.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
    budget: Budget,
    id: &AssetId,
) -> Result<Outcome, ImageError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| ImageError::NoSuchAsset { id: id.clone() })?;
    let brief = match Brief::of(project, root, asset) {
        Ok(brief) => brief,
        Err(why) => {
            return Ok(Outcome::Incomplete {
                why: why.to_string(),
            });
        }
    };
    let output = brief.output();

    // The cache first: a file already there is the answer to "has this been
    // paid for", whatever the document claims.
    if brief.realized(root) {
        let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
        record(project, id, &output, &bytes, estimate(&brief)?.cents);
        return Ok(Outcome::Cached { path: output });
    }

    let cents = estimate(&brief)?.cents;
    budget.check(cents)?;

    let bytes = match provider.draw(&brief) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(Outcome::Failed {
                message: error.message,
            });
        }
    };
    write(&output.resolve(root), &bytes)?;
    record(project, id, &output, &bytes, cents);
    Ok(Outcome::Generated {
        path: output,
        bytes: bytes.len(),
        estimated_cost_cents: cents,
    })
}

/// Points the asset at what is now on disk, clears any ticket, and says what
/// it was calculated to cost. **No `media` is written** — the commands probe
/// afterwards, for #249's reason: what a brief asked for is not a measurement.
///
/// A figure already recorded against this same file is kept: finding a
/// drawing again does not re-price it, and a still collected from a batch at
/// half price stays at half price.
pub(super) fn record(
    project: &mut Project,
    id: &AssetId,
    path: &ProjectPath,
    bytes: &[u8],
    cents: u64,
) {
    let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) else {
        return;
    };
    // A media block measured off a previous drawing describes a different
    // file; the probe that follows writes the new one.
    let same = asset.path.as_ref() == Some(path);
    if !same {
        asset.media = None;
    }
    if !same || asset.estimated_cost_cents.is_none() {
        asset.estimated_cost_cents = Some(cents);
    }
    asset.path = Some(path.clone());
    asset.state = Some(GenerationState::Generated);
    asset.operation = None;
    if !bytes.is_empty() {
        asset.sha256 = Some(hash_bytes(bytes));
    }
}

/// Writes the drawing, atomically, creating `generated/` if it is the first —
/// a truncated file named for its brief would be served as the answer for ever.
pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), ImageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ImageError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    scorsese_core::write::atomically(path, bytes).map_err(|source| ImageError::Write {
        path: path.to_path_buf(),
        source,
    })
}
