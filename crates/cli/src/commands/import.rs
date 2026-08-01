//! `scorsese import`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{AssetKind, Project, import_asset};
use scorsese_render::Ffprobe;

/// Copies the file into the project, probes it, and records it in the pool.
///
/// Importing the same media twice is not an error: it hashes to the asset that
/// already exists and comes back with that id, nothing copied. Which makes an
/// import loop safe to re-run — the thing an agent does by accident.
pub fn run(project_dir: &Path, file: &Path, kind: Option<AssetKind>) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let probe = Ffprobe::discover().context("ffprobe is needed to import media")?;

    let before = project.assets.len();
    let id = import_asset(&mut project, project_dir, file, kind, &probe)
        .with_context(|| format!("importing {}", file.display()))?;

    if project.assets.len() == before {
        println!("{id} — already in the pool, nothing copied");
        return Ok(());
    }

    project.save(project_dir).context("saving the project")?;
    let asset = project.asset(&id).expect("the asset just imported");
    println!("{id} — {:?}", asset.kind);
    if let Some(path) = &asset.path {
        println!("  {path}");
    }
    if let Some(media) = &asset.media {
        println!("  {}", super::probe::summarise(media));
    }
    Ok(())
}
