//! The lifecycle: sketch → queued → generated, and picking up where a run left
//! off.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, Project, ProjectPath, Timestamp};

use crate::credentials::Budget;
use crate::generated::{self, Plan, Prompted, write};
use crate::{prices, spending};

use super::{
    Brief, GenerationError, Outcome, Progress, Run, Stopped, Ticket, VideoProvider, submitted,
};

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
///
/// **A shot whose brief cannot be gathered is skipped, not fatal** — a
/// `first_image` still not drawn yet is the ordinary case, and the quote already
/// said it *would not be sent* ([`Outcome::Unready`]). What stops the run is
/// what makes the rest impossible or unwise — the ceiling, the provider refusing
/// a submission — and the [`Stopped`] it answers with carries the shots already
/// sent (#1057).
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, Stopped> {
    let ids = generated::ids::<Shots>(project);
    let mut done = Vec::new();
    let mut spent = 0;
    for (at, id) in ids.iter().enumerate() {
        // Against the ceiling *including what this run has already committed*.
        // A budget is built once, from what the project had spent before the
        // run started, so checking every shot against that same figure asks
        // twenty times whether one more shot fits and never whether twenty do.
        match one(project, root, provider, budget.spend(spent), id) {
            Ok(outcome) => {
                spent += outcome.spent_cents();
                done.push((id.clone(), outcome));
            }
            Err(error) => {
                return Err(Stopped {
                    unsent: generated::unsent::<Shots>(project, root, &ids[at..]),
                    run: Run {
                        outcomes: done,
                        spent_cents: spent,
                    },
                    error,
                });
            }
        }
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
    for id in generated::ids::<Shots>(project) {
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

/// Points every shot whose current brief's output is already on disk at that
/// file — the cache half of a run, with no provider, no key and nothing spent.
/// Which shots it found.
///
/// For a caller that keeps generated media somewhere other than the project's
/// own `generated/` — the hosted server keeps it in each user's library, found
/// by brief hash (#539) — and lays it out where each brief would land before
/// asking. What it finds is recorded exactly as a run records a cache hit,
/// ticket cleared included, so the document cannot say two different things
/// depending on which of the two noticed the file.
pub fn adopt(project: &mut Project, root: &Path) -> Vec<AssetId> {
    generated::adopt::<Shots>(project, root)
}

/// What a run would do with `asset` — see [`Plan`].
pub fn plan(project: &Project, root: &Path, asset: &Asset) -> Plan {
    generated::plan::<Shots>(project, root, asset)
}

/// Whether a run over this project's shots would have anything to do at all.
///
/// A dangling ticket counts even when the cache would answer it — the run
/// still clears it out of the document. An ungatherable brief counts too,
/// because the pass is where that refusal is voiced, naming what is missing.
pub fn pending(project: &Project, root: &Path) -> bool {
    generated::pending::<Shots>(project, root)
}

/// Shots, as [`generated`]'s shared steps see them.
pub(crate) struct Shots;

impl Prompted for Shots {
    const KIND: AssetKind = AssetKind::GeneratedVideo;
    type Brief = Brief;
    type Unready = GenerationError;
    type Error = GenerationError;

    fn brief(project: &Project, root: &Path, asset: &Asset) -> Result<Brief, GenerationError> {
        Brief::of(project, root, asset)
    }

    fn output(brief: &Brief) -> ProjectPath {
        brief.output()
    }

    fn realized(brief: &Brief, root: &Path) -> bool {
        brief.realized(root)
    }

    /// [`record`], and the ticket's sidecar forgotten: the file it was waiting
    /// for is here.
    fn keep(
        project: &mut Project,
        root: &Path,
        id: &AssetId,
        path: &ProjectPath,
        bytes: &[u8],
        brief: &Brief,
    ) -> Result<(), GenerationError> {
        record(project, id, path, bytes, brief);
        submitted::forget(root, id);
        Ok(())
    }
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
    let brief = match Brief::of(project, root, asset) {
        Ok(brief) => brief,
        // This shot's own problem, and the quote already said so: it is
        // reported and the run goes on with the rest.
        Err(why) => {
            return Ok(Outcome::Unready {
                why: why.to_string(),
            });
        }
    };

    // The cache, and it is checked before anything else on purpose: a file
    // already sitting there is the answer to "has this brief been paid for",
    // whatever the document happens to claim about the asset's state.
    if brief.realized(root) {
        let path = generated::found::<Shots>(project, root, id, &brief)?;
        return Ok(Outcome::Cached { path });
    }

    match project.asset(id).and_then(|asset| asset.operation.clone()) {
        Some(operation) => waiting(project, root, provider, id, &brief, Ticket(operation)),
        None => submit(project, root, provider, budget, id, &brief),
    }
}

/// Hands a brief over, and writes the ticket down before anything else can go
/// wrong.
fn submit(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    budget: Budget,
    id: &AssetId,
    brief: &Brief,
) -> Result<Outcome, GenerationError> {
    let estimate = prices::estimate(&brief.request)?;
    budget.check(estimate.cents)?;
    spending::readable(root)?;

    let ticket = provider.submit(brief)?;
    let Some(asset) = project.asset_mut(id) else {
        return Err(GenerationError::NoSuchAsset { id: id.clone() });
    };
    asset.operation = Some(ticket.0.clone());
    asset.queued_at = Timestamp::now();
    asset.state = Some(GenerationState::Queued);
    // Best effort, and after the ticket on purpose: the ticket is the record
    // that money was spent. Without the sidecar the video is filed under
    // whatever the brief is when it arrives, which is right unless it is
    // edited meanwhile — and a `generated/` that takes no sidecar takes no
    // video either, which is reported then.
    let _ = submitted::remember(root, id, &ticket.0, &brief.digest());
    // Paid for from here on, whether or not the video ever arrives (#1059).
    spending::sent(project, root, id, &brief.digest(), estimate.cents)?;
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
    let progress = match provider.poll(&ticket) {
        Ok(progress) => progress,
        // A ticket nobody will ever answer for is this shot's problem, not
        // the run's: stopping on it would stop every run in the project.
        Err(error) if error.lost => {
            release(project, root, id);
            return Ok(Outcome::Failed {
                message: format!("{} — the ticket is cleared", error.message),
            });
        }
        Err(error) => return Err(error.into()),
    };
    match progress {
        Progress::Waiting => Ok(still_waiting(project, root, id, &ticket)),
        // A rejected brief is a brief to edit and try again, and an asset
        // still claiming to be queued would be polled for ever.
        // Not charged for, either (#1109), so it stops counting.
        Progress::Failed(message) => {
            release(project, root, id);
            spending::refused(root, id)?;
            Ok(Outcome::Failed { message })
        }
        Progress::Ready(ready) => {
            let bytes = provider.fetch(&ready)?;
            let current = brief.output();
            let bought = submitted::output(root, id, &ticket.0).unwrap_or_else(|| current.clone());
            write(&bought.resolve(root), &bytes)?;
            if bought != current {
                release(project, root, id);
                return Ok(Outcome::Failed {
                    message: String::from(
                        "its brief changed while it was generating; the video from the old \
                         brief is kept in generated/ and costs nothing if that brief comes back",
                    ),
                });
            }
            Shots::keep(project, root, id, &current, &bytes, brief)?;
            Ok(Outcome::Generated {
                path: current,
                bytes: bytes.len(),
                estimated_cost_cents: prices::estimate(&brief.request)?.cents,
            })
        }
    }
}

/// Takes the ticket away and puts the state back: stale when an older take is
/// still on the asset, a sketch otherwise — so the shot is quoted and sent
/// again only when somebody asks, never by a sweep. The same rule a still
/// follows ([`crate::image`]).
fn release(project: &mut Project, root: &Path, id: &AssetId) {
    submitted::forget(root, id);
    if let Some(asset) = project.asset_mut(id) {
        asset.operation = None;
        asset.state = Some(if asset.path.is_some() {
            GenerationState::Stale
        } else {
            GenerationState::Sketch
        });
    }
}

/// Still in flight — and whether it has been in flight too long to still be
/// there when it finishes. An expired ticket is released: polling it again
/// can never bring the video back, and while it stood no tool could ask for
/// the shot again (#1106).
fn still_waiting(project: &mut Project, root: &Path, id: &AssetId, ticket: &Ticket) -> Outcome {
    let queued_at = project.asset(id).and_then(|asset| asset.queued_at.clone());
    let expired = queued_at
        .as_ref()
        .zip(cutoff())
        .is_some_and(|(queued, cutoff)| *queued < cutoff);
    if expired {
        release(project, root, id);
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

/// Records a shot through [`generated::record`], priced from its own brief —
/// what we worked out it would cost, at the moment of generating, so the table
/// cannot drift out from under it afterwards.
fn record(project: &mut Project, id: &AssetId, path: &ProjectPath, bytes: &[u8], brief: &Brief) {
    let cents = prices::estimate(&brief.request)
        .ok()
        .map(|estimate| estimate.cents);
    generated::record(project, id, path, bytes, cents);
}
