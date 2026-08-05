//! `scorsese generate`
//!
//! The command that spends money, and the only one in the binary that does.
//! Everything it prints is written on that assumption: what each brief did,
//! what the run is estimated to have cost, and what is still in flight.
//!
//! **Two providers, one ceiling, one total.** Shots and narration are separate
//! passes because they are separate vendors, but the budget is threaded from
//! the first into the second and the totals are added — a ceiling that each
//! pass checked independently would be a ceiling worth twice what somebody set.
//!
//! **A key is asked for only by a pass that has work.** Resolving both up front
//! would mean a project of nothing but narration could not be generated without
//! a Veo key, and the other way round. Which is what [`wants`] decides.

mod lines;
mod shots;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use scorsese_core::{Asset, AssetKind, Project, Reprobe, probe_assets};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::dollars;
use scorsese_providers::video::Run;
use scorsese_render::Ffprobe;

/// Realises every sketched brief, waiting up to `patience` for the shots.
pub(crate) fn run(project_dir: &Path, patience: Duration, dry_run: bool) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    if dry_run {
        return quote(&project);
    }

    let settings = Settings::load().unwrap_or_default();
    let budget = Budget::from_settings(&settings, spent_so_far(&project));

    // The document is saved whatever happens, and that is not tidiness: a
    // ticket written before a failure is the only record that money was spent,
    // and losing it means paying again for work already in flight.
    let (shots, lines, outcome) = passes(&mut project, project_dir, budget, patience);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    outcome?;

    let spoke = lines.iter().any(|(_, outcome)| {
        matches!(
            outcome,
            scorsese_providers::speech::Outcome::Generated { .. }
        )
    });
    if spoke {
        measure(&mut project, project_dir)?;
    }
    report(&shots, &lines);
    Ok(())
}

/// Both passes, each run only if it has something to do.
///
/// Returns what happened even when something failed, because the caller has to
/// save the project either way — a shot queued before an error is a shot that
/// has been paid for, and its ticket is the only record of it.
type Passes = (
    Run,
    Vec<(scorsese_core::AssetId, scorsese_providers::speech::Outcome)>,
    Result<()>,
);
fn passes(project: &mut Project, project_dir: &Path, budget: Budget, patience: Duration) -> Passes {
    let mut shots = Run {
        outcomes: Vec::new(),
        spent_cents: 0,
    };
    let mut lines = Vec::new();

    if wants(project, project_dir, AssetKind::GeneratedVideo) {
        match shots::pass(project, project_dir, budget, patience) {
            Ok(run) => shots = run,
            Err(error) => return (shots, lines, Err(error)),
        }
    }
    if wants(project, project_dir, AssetKind::GeneratedAudio) {
        // The ceiling carries across: what the shots committed is already spent
        // as far as the narration is concerned.
        match lines::pass(project, project_dir, budget.spend(shots.spent_cents)) {
            Ok(spoken) => lines = spoken,
            Err(error) => return (shots, lines, Err(error)),
        }
    }
    (shots, lines, Ok(()))
}

/// Whether this kind has anything left to do, and so whether its key is needed.
///
/// True when a brief of that kind has no file behind it yet, or has work in
/// flight. A project whose shots are all generated and present asks for no Veo
/// key at all — which is what makes a narration-only run possible for somebody
/// who has one vendor's key and not the other's.
fn wants(project: &Project, root: &Path, kind: AssetKind) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == kind)
        .any(|asset| asset.operation.is_some() || !is_present(asset, root))
}

/// Whether this asset's media is actually on disk.
///
/// The disk and not the document, deliberately. An asset can say `generated`
/// and have lost its file — deleted, or never copied along with the project —
/// and that one does need generating again, whatever the state field claims.
fn is_present(asset: &Asset, root: &Path) -> bool {
    asset
        .path
        .as_ref()
        .is_some_and(|path| path.resolve(root).is_file())
}

/// Picks up whatever finished while nobody was watching. Submits nothing.
pub(crate) fn sweep(project_dir: &Path) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let outcome = shots::sweep(&mut project, project_dir);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    let collected = outcome?;
    if collected.is_empty() {
        println!("Nothing is in flight.");
        return Ok(());
    }
    // A sweep spends nothing by construction, so its total is zero and saying
    // so is honest rather than a gap.
    shots::report(&Run {
        outcomes: collected,
        spent_cents: 0,
    });
    println!();
    println!(
        "About {} spent on this run — nothing, because a sweep only collects.",
        dollars(0)
    );
    Ok(())
}

/// Measures what was just spoken.
///
/// **Narration has no length until something looks.** A shot comes back exactly
/// as long as its request asked, so the provider fills that in from the brief;
/// how long a line takes depends on the words, and `scorsese-providers` has no
/// decoder to measure an MP3 with — nor should it grow one. The mix reads
/// `duration_seconds`, so an unprobed line is a line the mix skips. This is
/// where that gap is closed, above both crates, which is where it belongs.
fn measure(project: &mut Project, project_dir: &Path) -> Result<()> {
    let probe = Ffprobe::discover().context("ffprobe is needed to measure generated narration")?;
    probe_assets(project, project_dir, &probe, Reprobe::Skip);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))
}

/// What a run would cost, without a key and without spending anything.
fn quote(project: &Project) -> Result<()> {
    let total = shots::quote(project)? + lines::quote(project)?;
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

/// Every brief's line, then the one total that spans both providers.
fn report(shots: &Run, lines: &[(scorsese_core::AssetId, scorsese_providers::speech::Outcome)]) {
    if shots.outcomes.is_empty() && lines.is_empty() {
        println!("No generated assets in this project.");
        return;
    }
    shots::report(shots);
    lines::report(lines);

    let spent = shots.spent_cents + lines.iter().map(|(_, o)| o.spent_cents()).sum::<u64>();
    println!();
    println!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(spent)
    );
}
