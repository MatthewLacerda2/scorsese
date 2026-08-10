//! The shots half of `scorsese generate`: Veo, and what it costs.
//!
//! Split from [`super`] because there are two providers now and one file
//! holding both would be a file nobody reads to answer a question about
//! either. What is here is everything specific to *video* — the pass, and how
//! its outcomes read.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use scorsese_core::{AssetKind, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::prices::dollars;
use scorsese_providers::video::{Outcome, Plan, Run, VeoProvider, collect, generate_waiting, plan};

/// Realises every sketched shot, waiting up to `patience` for the results.
///
/// The key is resolved **here**, not by the caller, and only when this pass has
/// something to do — see [`super::pending`]. A project whose only generated
/// assets are narration must never be asked for a Veo key.
pub(super) fn pass(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
    patience: Duration,
) -> Result<Run> {
    let key = resolve(Provider::Gemini)?;
    let provider = VeoProvider::new(&key.secret);
    Ok(generate_waiting(
        project,
        project_dir,
        &provider,
        budget,
        patience,
        std::thread::sleep,
    )?)
}

/// Picks up whatever finished while nobody was watching. Submits nothing.
pub(super) fn sweep(
    project: &mut Project,
    project_dir: &Path,
) -> Result<Vec<(scorsese_core::AssetId, Outcome)>> {
    let key = resolve(Provider::Gemini)?;
    let provider = VeoProvider::new(&key.secret);
    Ok(collect(project, project_dir, &provider)?)
}

/// What this run would spend on shots, decided the way the run decides it.
///
/// Each asset is priced by its [`Plan`]: only a brief that would actually be
/// handed over reaches the total. A quote that priced every shot regardless
/// told somebody with three generated shots and one stale one that the run
/// would cost four shots' worth — a number the run could never spend.
pub(super) fn quote(project: &Project, root: &std::path::Path) -> Result<u64> {
    let mut total = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedVideo)
    {
        total += match plan(project, root, asset) {
            Plan::Realized(path) => {
                println!(
                    "{:<24} already generated — {path} — nothing to pay",
                    asset.id
                );
                0
            }
            Plan::InFlight => {
                println!(
                    "{:<24} still generating — already paid for, collected free",
                    asset.id
                );
                0
            }
            Plan::Unready(why) => {
                println!("{:<24} would not be sent — {why}", asset.id);
                0
            }
            Plan::Submit => {
                let estimate = scorsese_providers::prices::estimate(&asset.video_request())?;
                println!(
                    "{:<24} {} — {}s of {} at {}",
                    asset.id,
                    dollars(estimate.cents),
                    estimate.seconds,
                    asset.video_request().model.as_str(),
                    asset.video_request().resolution.as_str(),
                );
                estimate.cents
            }
        };
    }
    Ok(total)
}

/// One line per shot. The total is [`super`]'s, because it spans both
/// providers and a per-pass total would invite two of them being read as one.
pub(super) fn report(run: &Run) {
    for (id, outcome) in &run.outcomes {
        println!("{id:<24} {}", says(outcome));
    }
    let in_flight = run.in_flight();
    if in_flight > 0 {
        println!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — run `scorsese generate --collect` later to pick them up."
        );
    }
}

/// What one outcome reads as.
fn says(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Cached { path } => format!("already generated — {path}"),
        Outcome::Queued { operation, .. } => format!("queued — {operation}"),
        Outcome::Waiting { operation, .. } => format!("still generating — {operation}"),
        Outcome::Expired { queued_at, .. } => format!(
            "queued {} and past the {}-day window — the video is gone and the shot has to be \
             asked for again",
            queued_at
                .as_ref()
                .map_or_else(|| String::from("at some point"), ToString::to_string),
            scorsese_providers::video::RETENTION_DAYS
        ),
        Outcome::Generated { path, bytes, .. } => {
            format!("generated — {path} ({bytes} bytes)")
        }
        Outcome::Failed { message } => format!("refused — {message}"),
    }
}
