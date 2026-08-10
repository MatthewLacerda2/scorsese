//! The lifecycle: sketch → generated, in one call.
//!
//! There is no `queued` here and nothing to resume. A line is spoken on the
//! same connection the request went out on, so the states a Veo run has to
//! model — in flight, still waiting, past the retention window — describe
//! nothing that can happen to a narration.
//!
//! # What is an error, and what is an outcome
//!
//! The same split [`video`](crate::video) makes, drawn one step further along,
//! and the difference is deliberate. There, a brief that cannot be gathered —
//! no prompt, a still that will not read — stops the whole run. Here it does
//! not: an incomplete line is an [`Outcome::Incomplete`] and the other
//! nineteen are still spoken.
//!
//! That follows the reasoning the split already rests on. A run of twenty
//! narrations where one has no voice chosen yet is the *normal* state of a cut
//! being written, and stopping the run would mean re-speaking nineteen paid-for
//! lines to add the twentieth. What genuinely stops a run is what makes every
//! remaining line impossible: no key, the ceiling reached, a disk that will not
//! take a file.

use std::path::Path;

use scorsese_core::{
    Asset, AssetId, AssetKind, GenerationState, Project, ProjectPath, SpeechModel, hash_bytes,
};

use crate::credentials::Budget;
use crate::prices;

use super::{Brief, Outcome, SpeechError, SpeechProvider};

/// Speaks every line that needs it, and reports what happened to each.
///
/// **Idempotent by construction, and that is the money guarantee.** A line
/// whose file already exists is recorded and never sent. Calling this twice by
/// mistake costs nothing, which matters because the thing most likely to call
/// it twice is an agent that lost its connection midway.
///
/// The project is left describing what is on disk. Saving it is the caller's,
/// and the caller should save it **even when this returns an error** — a line
/// spoken before the failure is a line that has been paid for.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn SpeechProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, SpeechError> {
    let mut done = Vec::new();
    let mut spent = 0;
    for id in narration_ids(project) {
        let outcome = one(project, root, provider, budget, spent, &id)?;
        spent += outcome.spent_cents();
        done.push((id, outcome));
    }
    Ok(done)
}

/// What a run would do with one line, read without doing it.
///
/// The sibling of [`video::Plan`](crate::video::Plan), minus the in-flight
/// variant nothing here can be in. It exists for the callers that come
/// *before* a run — the dry-run quote, and the check that decides whether a
/// pass is worth resolving a key for — so they consult the same brief-derived
/// answer [`one`] will, rather than the asset's recorded `path`, which after
/// a rewrite still points at the previous reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The current brief's output is already in `generated/`. Nothing would be
    /// sent and nothing paid.
    Realized(
        /// Where it is, project-relative.
        ProjectPath,
    ),
    /// The line would be handed to the provider and billed.
    Submit,
    /// Not ready to be spoken, in [`Incomplete`](super::Incomplete)'s words —
    /// the ordinary state of a line somebody is still writing.
    Unready(
        /// What it is still missing.
        String,
    ),
}

/// What a run would do with `asset` — see [`Plan`].
pub fn plan(root: &Path, asset: &Asset) -> Plan {
    match Brief::of(asset) {
        Err(why) => Plan::Unready(why.to_string()),
        Ok(brief) if brief.realized(root) => Plan::Realized(brief.output()),
        Ok(_) => Plan::Submit,
    }
}

/// Whether a run over this project's narration would have anything to do.
///
/// An incomplete line counts, because the pass is where *not yet* is voiced,
/// naming the line and what it is missing.
pub fn pending(project: &Project, root: &Path) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedAudio)
        .any(|asset| !matches!(plan(root, asset), Plan::Realized(_)))
}

/// Every `generated_audio` asset, by id.
fn narration_ids(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedAudio)
        .map(|asset| asset.id.clone())
        .collect()
}

/// Speaks one line, or says why it was not spoken.
///
/// `already` is what this run has spent before reaching this line. It is passed
/// in rather than read back off the project because the project records what
/// each asset cost *when it was generated*, and a run that has just spent
/// forty cents must be held to the ceiling including those forty — otherwise
/// twenty lines each fitting under the ceiling individually would all go
/// through.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn SpeechProvider,
    budget: Budget,
    already: u64,
    id: &AssetId,
) -> Result<Outcome, SpeechError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| SpeechError::NoSuchAsset { id: id.clone() })?;

    // An incomplete line is this line's problem and nobody else's. That this
    // cannot stop the run is guaranteed by the signature rather than by the
    // handling here — `Brief::of` returns `Incomplete`, which has no fatal
    // variant to return.
    let brief = match Brief::of(asset) {
        Ok(brief) => brief,
        Err(why) => {
            return Ok(Outcome::Incomplete {
                why: why.to_string(),
            });
        }
    };
    let output = brief.output();

    // The cache, checked before anything else on purpose: a file already
    // sitting there is the answer to "has this been paid for", whatever the
    // document happens to claim about the asset's state.
    if brief.realized(root) {
        let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
        record(project, id, &output, &bytes, &brief)?;
        return Ok(Outcome::Cached { path: output });
    }

    let estimate = prices::speech(brief.request.model, brief.characters())?;
    budget.spend(already).check(estimate.cents)?;

    let bytes = match provider.speak(&brief) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(Outcome::Failed {
                message: error.message,
            });
        }
    };
    write(&output.resolve(root), &bytes)?;
    record(project, id, &output, &bytes, &brief)?;
    Ok(Outcome::Generated {
        path: output,
        bytes: bytes.len(),
        estimated_cost_cents: estimate.cents,
    })
}

/// Points the asset at what is now on disk, and says what it cost.
///
/// **No `media` is written**, which is what [`video`](crate::video) does too —
/// it briefly did otherwise, and #249 is what that cost. How long a line takes
/// depends on the words, and nothing here can measure an MP3: this crate has no
/// decoder and must not grow one. So the shape arrives when something probes
/// the file, which is why the commands that call this probe afterwards.
fn record(
    project: &mut Project,
    id: &AssetId,
    path: &ProjectPath,
    bytes: &[u8],
    brief: &Brief,
) -> Result<(), SpeechError> {
    let estimate = prices::speech(brief.request.model, brief.characters())?;
    let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) else {
        return Ok(());
    };
    asset.path = Some(path.clone());
    asset.state = Some(GenerationState::Generated);
    if !bytes.is_empty() {
        asset.sha256 = Some(hash_bytes(bytes));
    }
    // What we worked out it would cost, never what anybody billed — see
    // `prices`. Exact arithmetic here, and still an estimate: the rate is a
    // page somebody copied, and library-voice multipliers are ignored.
    asset.estimated_cost_cents = Some(estimate.cents);
    Ok(())
}

/// Writes the narration, creating `generated/` if this is the project's first.
fn write(path: &Path, bytes: &[u8]) -> Result<(), SpeechError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SpeechError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    // Atomic, and here the cache depends on it: a generation is named for the
    // hash of its brief, so a truncated file is indistinguishable from a
    // finished one and would be served as the answer for ever after — for a
    // line somebody paid for.
    scorsese_core::write::atomically(path, bytes).map_err(|source| SpeechError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// What a line of this model and length is expected to cost, in US cents.
///
/// Published so the quote a person approves and the figure the ceiling is
/// checked against are one calculation. Two would be two answers, and the first
/// person to notice they disagreed would be the one who had already paid.
pub fn quote(model: SpeechModel, characters: usize) -> u64 {
    prices::speech(model, characters).map_or(0, |estimate| estimate.cents)
}
