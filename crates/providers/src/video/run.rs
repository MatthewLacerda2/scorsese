//! The lifecycle: sketch → queued → generated, and picking up where a run left
//! off.

use std::path::Path;

use scorsese_core::{
    Asset, AssetId, AssetKind, GenerationState, Project, ProjectPath, Timestamp, hash_bytes,
};

use crate::credentials::Budget;
use crate::prices;

use super::{Brief, GenerationError, Outcome, Progress, Ticket, VideoProvider};

/// How long a finished generation stays fetchable.
///
/// Two days, and it is the vendor's window rather than ours: past it the video
/// is deleted, and the money was spent all the same. That is why a queued asset
/// is worth *reporting on* even when nobody asked — the cost of not noticing is
/// not a delay, it is the thing having to be paid for twice.
pub const RETENTION_DAYS: i64 = 2;

/// Seconds in a day.
const A_DAY: i64 = 86_400;

/// Submits every brief that needs it and collects everything that is ready.
///
/// **Idempotent by construction, and that is the money guarantee.** A brief
/// whose file already exists is recorded and never sent; an asset already in
/// flight is polled rather than submitted again. Calling this twice by mistake
/// costs nothing, which matters because the thing most likely to call it twice
/// is an agent that lost its connection midway.
///
/// The project is left describing what is on disk. Saving it is the caller's —
/// and the caller should save it **even when this returns an error**, because a
/// ticket written before the failure is the only record that money was spent.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, GenerationError> {
    let mut done = Vec::new();
    let mut spent = 0;
    for id in generated_video_ids(project) {
        // Against the ceiling *including what this run has already committed*.
        // A budget is built once, from what the project had spent before the
        // run started, so checking every shot against that same figure asks
        // twenty times whether one more shot fits and never whether twenty do.
        let outcome = one(project, root, provider, budget.spend(spent), &id)?;
        spent += outcome.spent_cents();
        done.push((id, outcome));
    }
    Ok(done)
}

/// Collects whatever finished while nobody was watching — and nothing else.
///
/// What runs on opening a project, and the reason waiting is not something a
/// process has to sit and do. It **never submits**, so it cannot spend money:
/// an unattended sweep that could start a generation is one that eventually
/// starts twenty.
pub fn collect(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
) -> Result<Vec<(AssetId, Outcome)>, GenerationError> {
    let mut done = Vec::new();
    for id in generated_video_ids(project) {
        let in_flight = project
            .asset(&id)
            .is_some_and(|asset| asset.operation.is_some());
        if !in_flight {
            continue;
        }
        let outcome = one(project, root, provider, Budget::unlimited(0), &id)?;
        done.push((id, outcome));
    }
    Ok(done)
}

/// What a run would do with one asset, read without doing it.
///
/// [`one`]'s decision order — the cache first, then the ticket, then a
/// submission — answered before any key is resolved and without spending
/// anything. It exists for the callers that come *before* a run: the dry-run
/// quote, and the check that decides whether a pass is worth resolving a key
/// for. Both used to consult the asset's recorded `path` instead, and that is
/// the file the *previous* generation landed in — after an edit it still
/// exists and still resolves, which is exactly how a stale shot got skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The current brief's output is already in `generated/`. Nothing would be
    /// sent and nothing paid.
    Realized(
        /// Where it is, project-relative.
        ProjectPath,
    ),
    /// A ticket is in flight. A run polls it and spends nothing new.
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
pub fn plan(project: &Project, root: &Path, asset: &Asset) -> Plan {
    let brief = match Brief::of(project, root, asset) {
        Ok(brief) => brief,
        Err(why) => return Plan::Unready(why.to_string()),
    };
    if brief.realized(root) {
        return Plan::Realized(brief.output());
    }
    if asset.operation.is_some() {
        return Plan::InFlight;
    }
    Plan::Submit
}

/// Whether a run over this project's shots would have anything to do at all.
///
/// A dangling ticket counts even when the cache would answer it — the run
/// still clears it out of the document. An ungatherable brief counts too,
/// because the pass is where that refusal is voiced, naming what is missing.
pub fn pending(project: &Project, root: &Path) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedVideo)
        .any(|asset| {
            asset.operation.is_some() || !matches!(plan(project, root, asset), Plan::Realized(_))
        })
}

/// Every `generated_video` asset, by id.
fn generated_video_ids(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedVideo)
        .map(|asset| asset.id.clone())
        .collect()
}

/// Moves one asset as far along as it can go in one pass.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    budget: Budget,
    id: &AssetId,
) -> Result<Outcome, GenerationError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| GenerationError::NoSuchAsset { id: id.clone() })?;
    let brief = Brief::of(project, root, asset)?;
    let output = brief.output();

    // The cache, and it is checked before anything else on purpose: a file
    // already sitting there is the answer to "has this brief been paid for",
    // whatever the document happens to claim about the asset's state.
    if brief.realized(root) {
        let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
        record(project, id, &output, &bytes, &brief);
        return Ok(Outcome::Cached { path: output });
    }

    match project.asset(id).and_then(|asset| asset.operation.clone()) {
        Some(operation) => waiting(project, root, provider, id, &brief, Ticket(operation)),
        None => submit(project, provider, budget, id, &brief),
    }
}

/// Hands a brief over, and writes the ticket down before anything else can go
/// wrong.
fn submit(
    project: &mut Project,
    provider: &dyn VideoProvider,
    budget: Budget,
    id: &AssetId,
    brief: &Brief,
) -> Result<Outcome, GenerationError> {
    let estimate = prices::estimate(&brief.request)?;
    budget.check(estimate.cents)?;

    let ticket = provider.submit(brief)?;
    let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) else {
        return Err(GenerationError::NoSuchAsset { id: id.clone() });
    };
    asset.operation = Some(ticket.0.clone());
    asset.queued_at = Timestamp::now();
    asset.state = Some(GenerationState::Queued);
    Ok(Outcome::Queued {
        operation: ticket.0,
        estimated_cost_cents: estimate.cents,
    })
}

/// Asks after a ticket and does whatever the answer calls for.
fn waiting(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    id: &AssetId,
    brief: &Brief,
    ticket: Ticket,
) -> Result<Outcome, GenerationError> {
    match provider.poll(&ticket)? {
        Progress::Waiting => Ok(still_waiting(project, id, &ticket)),
        Progress::Failed(message) => {
            // The ticket goes, and the state goes back to what it was: a
            // rejected brief is a brief to edit and try again, and an asset
            // still claiming to be queued would be polled for ever.
            if let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) {
                asset.operation = None;
                asset.state = Some(GenerationState::Sketch);
            }
            Ok(Outcome::Failed { message })
        }
        Progress::Ready(ready) => {
            let bytes = provider.fetch(&ready)?;
            let output = brief.output();
            write(&output.resolve(root), &bytes)?;
            record(project, id, &output, &bytes, brief);
            Ok(Outcome::Generated {
                path: output,
                bytes: bytes.len(),
                estimated_cost_cents: prices::estimate(&brief.request)?.cents,
            })
        }
    }
}

/// Still in flight — and whether it has been in flight too long to still be
/// there when it finishes.
fn still_waiting(project: &Project, id: &AssetId, ticket: &Ticket) -> Outcome {
    let queued_at = project.asset(id).and_then(|asset| asset.queued_at.clone());
    let expired = queued_at
        .as_ref()
        .zip(cutoff())
        .is_some_and(|(queued, cutoff)| *queued < cutoff);
    if expired {
        Outcome::Expired {
            operation: ticket.0.clone(),
            queued_at,
        }
    } else {
        Outcome::Waiting {
            operation: ticket.0.clone(),
            queued_at,
        }
    }
}

/// The stamp before which a queued generation is past the vendor's window.
fn cutoff() -> Option<Timestamp> {
    Timestamp::from_unix(Timestamp::unix_now()? - RETENTION_DAYS * A_DAY)
}

/// Points the asset at what is now on disk, and says what it cost.
///
/// **No `media` is written**, and it used to be. Two of those fields a brief
/// does know — Veo delivers exactly the length and the raster it was asked for
/// — so this filled them in from the request as a shortcut, on the promise that
/// a probe would overwrite them with what ffprobe found. **That promise was
/// false.** A probe skips any asset that already carries a `media` block, so
/// the fields the brief could *not* know stayed unknown for ever. One of them
/// is `audio_channels`, and Veo returns video with sound on it — engines,
/// shouting, a room — which the render then read as absent and planned silence
/// for. See #249.
///
/// So the shape arrives when something measures the file, exactly as a spoken
/// line's length does, and for the same reason: what a brief *asked for* is not
/// a measurement and must not be written where measurements live. The commands
/// that call this probe afterwards.
fn record(project: &mut Project, id: &AssetId, path: &ProjectPath, bytes: &[u8], brief: &Brief) {
    let estimate = prices::estimate(&brief.request).ok();
    let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) else {
        return;
    };
    asset.path = Some(path.clone());
    asset.state = Some(GenerationState::Generated);
    asset.operation = None;
    if !bytes.is_empty() {
        asset.sha256 = Some(hash_bytes(bytes));
    }
    // What we worked out it would cost, never what anybody billed — see
    // `prices`. Written at the moment of generating so the table cannot drift
    // out from under it afterwards.
    asset.estimated_cost_cents = estimate.map(|estimate| estimate.cents);
}

/// Writes the generation, creating `generated/` if this is the project's first.
fn write(path: &Path, bytes: &[u8]) -> Result<(), GenerationError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GenerationError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    // Atomic, and here the cache depends on it: a generation is named for the
    // hash of its brief, so a truncated file is indistinguishable from a
    // finished one and would be served as the answer for ever after — for a
    // shot somebody paid for.
    scorsese_core::write::atomically(path, bytes).map_err(|source| GenerationError::Write {
        path: path.to_path_buf(),
        source,
    })
}
