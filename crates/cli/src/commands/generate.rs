//! `scorsese generate`
//!
//! The command that spends money, and the only one in the binary that does.
//! Everything it prints is written on that assumption: what each shot did, what
//! the run is estimated to have cost, and what is still in flight.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use scorsese_core::Project;
use scorsese_providers::credentials::{Budget, Provider, Settings, resolve};
use scorsese_providers::prices::dollars;
use scorsese_providers::video::{Outcome, Run, VeoProvider, collect, generate_waiting};

/// Realises every sketched shot, waiting up to `patience` for the results.
pub(crate) fn run(project_dir: &Path, patience: Duration, dry_run: bool) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    if dry_run {
        return quote(&project);
    }

    let key = resolve(Provider::Gemini)?;
    let settings = Settings::load().unwrap_or_default();
    let budget = Budget::from_settings(&settings, spent_so_far(&project));
    let provider = VeoProvider::new(&key.secret);

    // The document is saved whatever happens, and that is not tidiness: a
    // ticket written before a failure is the only record that money was spent,
    // and losing it means paying again for work already in flight.
    let outcome = generate_waiting(
        &mut project,
        project_dir,
        &provider,
        budget,
        patience,
        std::thread::sleep,
    );
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    report(&outcome?);
    Ok(())
}

/// Picks up whatever finished while nobody was watching. Submits nothing.
pub(crate) fn sweep(project_dir: &Path) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let key = resolve(Provider::Gemini)?;
    let provider = VeoProvider::new(&key.secret);

    let outcome = collect(&mut project, project_dir, &provider);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    let collected = outcome?;
    if collected.is_empty() {
        println!("Nothing is in flight.");
    } else {
        // A sweep spends nothing by construction, so its total is zero and
        // saying so is honest rather than a gap.
        report(&Run {
            outcomes: collected,
            spent_cents: 0,
        });
    }
    Ok(())
}

/// What a run would cost, without a key and without spending anything.
fn quote(project: &Project) -> Result<()> {
    let mut total = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind.is_prompted())
    {
        let estimate = scorsese_providers::prices::estimate(&asset.video_request())?;
        println!(
            "{:<24} {} — {}s of {} at {}",
            asset.id,
            dollars(estimate.cents),
            estimate.seconds,
            asset.video_request().model.as_str(),
            asset.video_request().resolution.as_str(),
        );
        total += estimate.cents;
    }
    println!();
    println!(
        "About {} for the whole run — calculated from the published rates, never a bill. \
         See docs/prices.md.",
        dollars(total)
    );
    Ok(())
}

/// What the assets already say has been spent on them.
fn spent_so_far(project: &Project) -> u64 {
    project
        .assets
        .iter()
        .filter_map(|asset| asset.estimated_cost_cents)
        .sum()
}

/// One line per shot, then the total.
fn report(run: &Run) {
    if run.outcomes.is_empty() {
        println!("No generated_video assets in this project.");
        return;
    }
    for (id, outcome) in &run.outcomes {
        println!("{id:<24} {}", says(outcome));
    }
    println!();
    println!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(run.spent_cents)
    );
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
