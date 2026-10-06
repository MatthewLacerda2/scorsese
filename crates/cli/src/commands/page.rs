//! `scorsese page`

use std::io::Read as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{AssetId, AssetKind, Project, write_page};

/// Writes the page called `id` from `file` — `-` is standard input — making
/// the `html` asset when nothing answers to the id: the `page_write` MCP tool.
pub(crate) fn write(project_dir: &Path, id: &str, file: &Path) -> Result<()> {
    let html = if file == Path::new("-") {
        let mut html = String::new();
        std::io::stdin()
            .read_to_string(&mut html)
            .context("reading the page from standard input")?;
        html
    } else {
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?
    };
    let mut project = open(project_dir)?;
    let written = write_page(&mut project, project_dir, id, &html)?;
    if written.created {
        project.save(project_dir).context("saving the project")?;
        println!("{} — html, new", written.id);
    } else {
        println!("{} — rewritten", written.id);
    }
    println!("  {} ({} bytes)", written.path, html.len());
    Ok(())
}

/// Prints the page called `id` as it is on disk: the `page_read` MCP tool.
pub(crate) fn read(project_dir: &Path, id: &str) -> Result<()> {
    let project = open(project_dir)?;
    let asset = project
        .asset(&AssetId::new(id))
        .with_context(|| format!("there is no asset `{id}`"))?;
    let (AssetKind::Html, Some(path)) = (asset.kind, &asset.path) else {
        bail!("`{id}` is not a page");
    };
    path.check()
        .map_err(|problem| anyhow::anyhow!("the page `{id}` is at {path}, which {problem}"))?;
    let html = std::fs::read_to_string(path.resolve(project_dir))
        .with_context(|| format!("reading the page `{id}` at {path}"))?;
    print!("{html}");
    Ok(())
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}
