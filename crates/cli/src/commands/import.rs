//! `scorsese import`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{AssetKind, Import, Project, import_path};
use scorsese_render::Ffprobe;

/// Copies media into the project, probes it, and records it in the pool.
///
/// A file brings in one asset; a directory brings in the media directly inside
/// it, one asset each. Importing the same media twice is not an error: it
/// hashes to the asset that already exists and comes back with that id,
/// nothing copied. Which makes an import loop safe to re-run — the thing an
/// agent does by accident.
pub(crate) fn run(project_dir: &Path, path: &Path, kind: Option<AssetKind>) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let probe = Ffprobe::discover().context("ffprobe is needed to import media")?;

    let report = import_path(&mut project, project_dir, path, kind, &probe)
        .with_context(|| format!("importing {}", path.display()))?;

    if report.imported.iter().any(|one| !one.reused) {
        project.save(project_dir).context("saving the project")?;
    }
    print(&project, &report);
    Ok(())
}

/// What came in, then what did not. The skips go last because they are the
/// part a reader has to act on, and burying them above a list of successes is
/// how a licence file quietly stands in for a mistyped video.
fn print(project: &Project, report: &Import) {
    for one in &report.imported {
        if one.reused {
            println!("{} — already in the pool, nothing copied", one.id);
            continue;
        }
        let Some(asset) = project.asset(&one.id) else {
            continue;
        };
        println!("{} — {:?}", one.id, asset.kind);
        if let Some(path) = &asset.path {
            println!("  {path}");
        }
        if let Some(media) = &asset.media {
            println!("  {media}");
        }
    }
    for skipped in &report.skipped {
        println!("{} — skipped: {}", skipped.source, skipped.why);
    }
}
