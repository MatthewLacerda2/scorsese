//! `scorsese sequence`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{
    AssetId, Frames, ImageSequence, Project, SequenceChange, SequenceImport, change_sequence,
    import_sequence,
};
use scorsese_render::Ffprobe;

/// Brings a folder of frames in as one image sequence, then applies a hold or
/// a loop if either was asked for — the same two steps the `sequence` MCP tool
/// and the `import` tool's `sequence` flag take.
pub(crate) fn import(
    project_dir: &Path,
    dir: &Path,
    hold: Option<u64>,
    looping: bool,
) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let probe = Ffprobe::discover().context("ffprobe is needed to import media")?;
    let report = import_sequence(&mut project, project_dir, dir, &probe)
        .with_context(|| format!("importing {}", dir.display()))?;
    let change = SequenceChange {
        hold: hold.map(Frames),
        looping: looping.then_some(true),
        ..SequenceChange::default()
    };
    let sequence = change_sequence(&mut project, &report.id, change)
        .context("timing the sequence")?
        .after;
    project.save(project_dir).context("saving the project")?;
    print!("{}", imported(&report, &sequence));
    Ok(())
}

/// Makes a sequence from stills already in the pool, or changes one.
pub(crate) fn set(
    project_dir: &Path,
    asset: &str,
    stills: Option<Vec<String>>,
    hold: Option<u64>,
    looping: Option<bool>,
) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let change = SequenceChange {
        stills: stills.map(|ids| ids.into_iter().map(AssetId::new).collect()),
        hold: hold.map(Frames),
        looping,
    };
    let id = AssetId::new(asset);
    let changed = change_sequence(&mut project, &id, change)?;
    project.save(project_dir).context("saving the project")?;
    match &changed.before {
        None => println!("{id} — made: {}", changed.after),
        Some(before) => println!("{id} — was {}; now {}", before, changed.after),
    }
    Ok(())
}

/// What an import brought in, in the order a reader acts on it: the asset to
/// put on a clip, then what to check — the gaps and the skipped files.
fn imported(report: &SequenceImport, sequence: &ImageSequence) -> String {
    let mut lines = Vec::new();
    if report.existed {
        lines.push(format!(
            "{} — already in the pool, nothing copied",
            report.id
        ));
    } else {
        let copied = report.stills.len() - report.reused;
        lines.push(format!(
            "{} — image_sequence: {} ({copied} copied, {} already in the pool)",
            report.id, sequence, report.reused
        ));
    }
    for gap in &report.gaps {
        lines.push(format!(
            "  gap: {} missing between {} and {}",
            gap.missing, gap.after, gap.before
        ));
    }
    for skipped in &report.skipped {
        lines.push(format!("{} — skipped: {}", skipped.source, skipped.why));
    }
    lines.join("\n") + "\n"
}
