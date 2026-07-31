//! `scorsese probe` — find out what the pool is actually made of.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{MediaMetadata, ProbeOutcome, Probed, Project, Reprobe, probe_assets};
use scorsese_render::Ffprobe;

/// Probes every asset with a file and no metadata, and writes what came back
/// into the assets table.
///
/// Safe to re-run, and meant to be: an asset that has already been probed is
/// left alone, so this costs one pass over the assets table on a pool that is
/// up to date. `--all` is the way to say the recorded metadata is wrong and
/// every file should be read again.
pub fn run(project_dir: &Path, all: bool) -> Result<()> {
    let mut project = open(project_dir)?;
    let probe = Ffprobe::discover().context("ffprobe is needed to probe media")?;
    let reprobe = if all { Reprobe::All } else { Reprobe::Skip };

    let report = probe_assets(&mut project, project_dir, &probe, reprobe);
    if report.is_empty() {
        println!("No assets with a file to probe.");
        return Ok(());
    }

    let recorded = tally(&report, |outcome| *outcome == ProbeOutcome::Recorded);
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
            .map_or_else(|| "probed".to_owned(), summarise),
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

fn tally(report: &[Probed], counts: impl Fn(&ProbeOutcome) -> bool) -> usize {
    report.iter().filter(|row| counts(&row.outcome)).count()
}

/// A media block in one readable line: the shape, the rate, the length.
///
/// Shared with `scorsese import`, which says the same thing about the one file
/// it just brought in. Two commands that report the same metadata differently
/// would read like two different facts.
pub(crate) fn summarise(media: &MediaMetadata) -> String {
    let mut parts = Vec::new();
    if let (Some(width), Some(height)) = (media.width, media.height) {
        parts.push(format!("{width}x{height}"));
    }
    if let Some(rate) = media.frame_rate {
        parts.push(format!("{rate} fps"));
    }
    if let Some(duration) = media.duration_seconds {
        parts.push(format!("{duration:.2}s"));
    }
    if let Some(channels) = media.audio_channels {
        parts.push(format!("{channels} audio ch"));
    }
    if parts.is_empty() {
        return "no metadata reported".to_owned();
    }
    parts.join(", ")
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}
