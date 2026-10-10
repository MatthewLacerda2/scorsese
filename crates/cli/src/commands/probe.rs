//! `scorsese probe` — find out what the pool is actually made of.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{ProbeOutcome, Probed, Project, Reprobe, probe_assets};
use scorsese_render::Ffprobe;

/// Probes every asset with a file and no metadata, and writes what came back
/// into the assets table.
///
/// Safe to re-run, and meant to be: an asset that has already been probed is
/// left alone, so this costs one pass over the assets table on a pool that is
/// up to date. `--all` is the way to say the recorded metadata is wrong and
/// every file should be read again.
///
/// Opens a project whose only problem is a stale measurement — a clip longer
/// than the length recorded for a file that has since grown (#1007) — because
/// measuring is what fixes it. The document is validated after the probe and
/// before the save, so what is written is always a project that loads.
pub(crate) fn run(project_dir: &Path, all: bool) -> Result<()> {
    let mut project = open(project_dir)?;
    let probe = Ffprobe::discover().context("ffprobe is needed to probe media")?;
    let reprobe = if all { Reprobe::All } else { Reprobe::Skip };

    let report = probe_assets(&mut project, project_dir, &probe, reprobe);
    if report.is_empty() {
        println!("No assets with a file to probe.");
        return Ok(());
    }

    let recorded = tally(&report, |outcome| *outcome == ProbeOutcome::Recorded);
    if let Err(problems) = project.validate() {
        anyhow::bail!(
            "measured, nothing saved: {problems}{}",
            if all { "" } else { RETRY }
        );
    }
    if recorded > 0 {
        project.save(project_dir).context("saving the project")?;
    }
    for row in &report {
        if let Some(line) = line(&project, row) {
            println!("{line}");
        }
    }
    println!("\n{}", summary(&report, recorded, all));
    Ok(())
}

/// One line per asset that changed or is in trouble.
///
/// An asset that was already known produces nothing at all. Re-running this
/// after everything is probed should say "nothing to do" in as few words as
/// possible — a wall of `already known` on every run is how a report stops
/// being read.
fn line(project: &Project, row: &Probed) -> Option<String> {
    let said = match &row.outcome {
        ProbeOutcome::Recorded => project
            .asset(&row.id)
            .and_then(|asset| asset.media.as_ref())
            .map_or_else(|| "probed".to_owned(), ToString::to_string),
        ProbeOutcome::AlreadyKnown => return None,
        ProbeOutcome::Missing => "FILE MISSING".to_owned(),
        ProbeOutcome::Failed(why) => format!("COULD NOT PROBE: {why}"),
    };
    Some(format!("{:<20} {said}", row.id))
}

/// The closing line: what was done, and what to do next if anything is left.
fn summary(report: &[Probed], recorded: usize, all: bool) -> String {
    let known = tally(report, |outcome| *outcome == ProbeOutcome::AlreadyKnown);
    let missing = tally(report, |outcome| *outcome == ProbeOutcome::Missing);
    let failed = report.len() - recorded - known - missing;

    let mut said = format!("{recorded} probed");
    if known > 0 {
        said.push_str(&format!(", {known} already known"));
    }
    if missing > 0 {
        said.push_str(&format!(", {missing} with no file"));
    }
    if failed > 0 {
        said.push_str(&format!(", {failed} unreadable"));
    }
    if known > 0 && !all {
        said.push_str("\n(already-probed assets are left alone — pass --all to read them again)");
    }
    said
}

/// What a refusal adds when the measurement it rests on may be the stale one.
const RETRY: &str = "\n(already-probed assets were left alone — `probe --all` measures them again)";

fn tally(report: &[Probed], counts: impl Fn(&ProbeOutcome) -> bool) -> usize {
    report.iter().filter(|row| counts(&row.outcome)).count()
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load_to_measure(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}
