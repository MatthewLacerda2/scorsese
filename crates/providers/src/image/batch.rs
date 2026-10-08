//! Ordering stills as a half-price batch (#894): sketch → queued.
//!
//! [`video::submit`](crate::video)'s half of a shot's lifecycle, for many
//! stills at once: every brief a run would draw is handed over in as few
//! batch jobs as the vendor allows, and each still's ticket — its job's name —
//! is written into the document before anything else can go wrong. A later
//! run collects them ([`collect`](super::collect)).
//!
//! **One job per model, split by weight.** The Batch API is called per model,
//! so stills on two models are two jobs; and inline requests may weigh 20 MB a
//! job, so a set heavy with reference pictures is split into as many as it
//! takes. Each job is ordered on its own, so one refused leaves the others
//! ordered.

use std::path::Path;

use scorsese_core::{AssetId, GenerationState, ImageModel, Project, Timestamp};

use crate::api::gemini::batch::INLINE_LIMIT;
use crate::credentials::Budget;
use crate::prices;

use super::run::{record, still_ids};
use super::{Brief, ImageError, ImageProvider, Outcome, collect};

/// Orders every still that needs drawing as a half-price batch, after
/// collecting whatever earlier batches have finished — and reports what
/// happened to each.
///
/// The money guarantee is [`generate`](super::generate)'s: a still whose file
/// is already there is recorded and never ordered, and one already waiting in
/// a batch is asked after rather than ordered twice. Saving the project is
/// the caller's, **even when this returns an error**: a ticket written before
/// it is the only record of a batch that will be billed.
pub fn batch(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, ImageError> {
    let mut done = collect(project, root, provider)?;
    let mut briefs = Vec::new();
    for id in still_ids(project) {
        if done.iter().any(|(handled, _)| handled == &id) {
            continue;
        }
        let Some(asset) = project.asset(&id) else {
            continue;
        };
        match Brief::of(project, root, asset) {
            Err(why) => done.push((
                id,
                Outcome::Incomplete {
                    why: why.to_string(),
                },
            )),
            Ok(brief) if brief.realized(root) => {
                let output = brief.output();
                let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
                record(project, &id, &output, &bytes, half(&brief)?);
                done.push((id, Outcome::Cached { path: output }));
            }
            Ok(brief) => briefs.push(brief),
        }
    }
    let mut spent = 0;
    for job in jobs(&briefs) {
        let cents = job
            .iter()
            .map(|brief| half(brief))
            .sum::<Result<u64, _>>()?;
        budget.spend(spent).check(cents)?;
        match provider.order(&job) {
            Ok(operation) => {
                spent += cents;
                for brief in job {
                    queue(project, &brief.id, &operation);
                    let estimated_cost_cents = half(brief)?;
                    done.push((
                        brief.id.clone(),
                        Outcome::Ordered {
                            operation: operation.clone(),
                            estimated_cost_cents,
                        },
                    ));
                }
            }
            Err(error) => {
                for brief in job {
                    let message = error.message.clone();
                    done.push((brief.id.clone(), Outcome::Failed { message }));
                }
            }
        }
    }
    Ok(done)
}

/// What one still costs in a batch — the one figure the quote, the ceiling
/// and the record all use.
pub(super) fn half(brief: &Brief) -> Result<u64, ImageError> {
    let priced = prices::image_in_batch(
        &brief.request,
        brief.characters(),
        brief.reference_images.len(),
    )?;
    Ok(priced.cents)
}

/// The briefs as jobs: one model each, each under the inline limit — in
/// document order within a job, and jobs in the order their models first
/// appear.
fn jobs(briefs: &[Brief]) -> Vec<Vec<&Brief>> {
    let mut models: Vec<ImageModel> = Vec::new();
    for brief in briefs {
        if !models.contains(&brief.request.model) {
            models.push(brief.request.model);
        }
    }
    let mut jobs = Vec::new();
    for model in models {
        let mut job: Vec<&Brief> = Vec::new();
        let mut weight = 0;
        for brief in briefs.iter().filter(|brief| brief.request.model == model) {
            let heavy = weight_of(brief);
            if !job.is_empty() && weight + heavy > INLINE_LIMIT {
                jobs.push(std::mem::take(&mut job));
                weight = 0;
            }
            job.push(brief);
            weight += heavy;
        }
        if !job.is_empty() {
            jobs.push(job);
        }
    }
    jobs
}

/// About how many bytes one brief adds to a job's body: its prompt, its
/// pictures in base64 (four bytes for every three), and the JSON around them.
pub(super) fn weight_of(brief: &Brief) -> usize {
    let pictures: usize = brief
        .reference_images
        .iter()
        .map(|still| still.bytes.len().div_ceil(3) * 4 + 128)
        .sum();
    brief.prompt.len() + pictures + 512
}

/// Writes the ticket down: queued, in `operation`, from now.
fn queue(project: &mut Project, id: &AssetId, operation: &str) {
    if let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) {
        asset.operation = Some(operation.to_owned());
        asset.queued_at = Timestamp::now();
        asset.state = Some(GenerationState::Queued);
    }
}
