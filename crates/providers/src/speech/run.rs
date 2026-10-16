//! The lifecycle: sketch → generated, in one call.
//!
//! There is no `queued` here and nothing to resume. A line is spoken on the
//! same connection the request went out on, so the states a Veo run has to
//! model — in flight, still waiting, past the retention window — describe
//! nothing that can happen to a narration.
//!
//! # What is an error, and what is an outcome
//!
//! The same split [`video`](crate::video) and [`image`](crate::image) make: a
//! brief that cannot be gathered is that asset's outcome, not the end of the
//! run. An incomplete line is an [`Outcome::Incomplete`] and the other
//! nineteen are still spoken.
//!
//! The reasoning is the split's own. A run of twenty
//! narrations where one has no voice chosen yet is the *normal* state of a cut
//! being written, and stopping the run would mean re-speaking nineteen paid-for
//! lines to add the twentieth. What genuinely stops a run is what makes every
//! remaining line impossible: no key, the ceiling reached, a disk that will not
//! take a file.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath, SpeechModel};

use scorsese_core::words::Words;

use crate::credentials::Budget;
use crate::generated::{self, Prompted, write};
use crate::prices;

use super::{Brief, Incomplete, Outcome, SpeechError, SpeechProvider, Spoken, Stopped};

/// Speaks every line that needs it, and reports what happened to each.
///
/// **Idempotent by construction, and that is the money guarantee.** A line
/// whose file already exists is recorded and never sent. Calling this twice by
/// mistake costs nothing, which matters because the thing most likely to call
/// it twice is an agent that lost its connection midway.
///
/// The project is left describing what is on disk. Saving it is the caller's,
/// and the caller should save it **even when this returns an error** — a line
/// spoken before the failure is a line that has been paid for, and the
/// [`Stopped`] it answers with lists it beside the reason (#1153).
///
/// Each line is held to the ceiling **including what this run has already
/// spent**, passed in rather than read back off the project: the project
/// records what each asset cost *when it was generated*, and a run that has
/// just spent forty cents must be held to the ceiling including those forty —
/// otherwise twenty lines each fitting under the ceiling individually would
/// all go through.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn SpeechProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, Stopped> {
    let ids = generated::ids::<Lines>(project);
    let mut done = Vec::new();
    let mut spent = 0;
    for (at, id) in ids.iter().enumerate() {
        match one(project, root, provider, budget.spend(spent), id) {
            Ok(outcome) => {
                spent += outcome.spent_cents();
                done.push((id.clone(), outcome));
            }
            Err(error) => {
                return Err(Stopped {
                    unsent: generated::unsent::<Lines>(project, root, &ids[at..]),
                    run: done,
                    error,
                });
            }
        }
    }
    Ok(done)
}

/// Points every line whose current brief's output is already on disk at that
/// file — the cache half of a run, with no provider, no key and nothing spent.
/// Which lines it found.
///
/// The sibling of [`video::adopt`](crate::video::adopt), for the same caller:
/// one that keeps generated media outside the project and lays it out where
/// each brief would land before asking.
pub fn adopt(project: &mut Project, root: &Path) -> Vec<AssetId> {
    generated::adopt::<Lines>(project, root)
}

/// What a run would do with one line, read without doing it.
///
/// The sibling of [`video::Plan`](crate::video::Plan), minus the in-flight
/// variant nothing here can be in. It exists for the callers that come
/// *before* a run — the [quote](crate::quote), and the check that decides whether a
/// pass is worth resolving a key for — so they consult the same brief-derived
/// answer the pass will, rather than the asset's recorded `path`, which after
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

/// Lines, as [`generated`]'s shared steps see them.
struct Lines;

impl Prompted for Lines {
    const KIND: AssetKind = AssetKind::GeneratedAudio;
    type Brief = Brief;
    type Unready = Incomplete;
    type Error = SpeechError;

    fn brief(_project: &Project, _root: &Path, asset: &Asset) -> Result<Brief, Incomplete> {
        Brief::of(asset)
    }

    fn output(brief: &Brief) -> ProjectPath {
        brief.output()
    }

    fn realized(brief: &Brief, root: &Path) -> bool {
        brief.realized(root)
    }

    fn keep(
        project: &mut Project,
        _root: &Path,
        id: &AssetId,
        path: &ProjectPath,
        bytes: &[u8],
        brief: &Brief,
    ) -> Result<(), SpeechError> {
        record(project, id, path, bytes, brief)
    }
}

/// Speaks one line, or says why it was not spoken. `budget` already counts
/// what this run has spent before reaching it.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn SpeechProvider,
    budget: Budget,
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

    // The cache, checked before anything else on purpose: a file already
    // sitting there is the answer to "has this been paid for", whatever the
    // document happens to claim about the asset's state.
    if brief.realized(root) {
        let path = generated::found::<Lines>(project, root, id, &brief)?;
        return Ok(Outcome::Cached { path });
    }
    let output = brief.output();

    let estimate = prices::speech(brief.request.model, brief.characters())?;
    budget.check(estimate.cents)?;

    let Spoken {
        audio: bytes,
        words,
    } = match provider.speak(&brief) {
        Ok(spoken) => spoken,
        Err(error) => {
            return Ok(Outcome::Failed {
                message: error.message,
            });
        }
    };
    // The timings before the audio: the audio being there is what says the
    // line is done, so once it is, whatever timings it came with are too.
    if let Some(words) = words {
        let beside = Words::beside(&output).resolve(root);
        write(&beside, words.to_json().as_bytes())?;
    }
    write(&output.resolve(root), &bytes)?;
    record(project, id, &output, &bytes, &brief)?;
    Ok(Outcome::Generated {
        path: output,
        bytes: bytes.len(),
        estimated_cost_cents: estimate.cents,
    })
}

/// Records a line through [`generated::record`], priced from its own brief.
/// Exact arithmetic, and still an estimate: the rate is a page somebody
/// copied, and library-voice multipliers are ignored.
fn record(
    project: &mut Project,
    id: &AssetId,
    path: &ProjectPath,
    bytes: &[u8],
    brief: &Brief,
) -> Result<(), SpeechError> {
    let estimate = prices::speech(brief.request.model, brief.characters())?;
    generated::record(project, id, path, bytes, Some(estimate.cents));
    Ok(())
}

/// What a line of this model and length is expected to cost, in US cents.
///
/// Published so the quote a person approves and the figure the ceiling is
/// checked against are one calculation. Two would be two answers, and the first
/// person to notice they disagreed would be the one who had already paid.
pub fn quote(model: SpeechModel, characters: usize) -> u64 {
    prices::speech(model, characters).map_or(0, |estimate| estimate.cents)
}
