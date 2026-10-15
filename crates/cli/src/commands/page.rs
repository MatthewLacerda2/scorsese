//! `scorsese page`

use std::io::Read as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{AssetId, PageReadError, read_page, write_page};

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
    let mut project = super::open(project_dir)?;
    let written = write_page(&mut project, project_dir, id, &html)?;
    if written.created {
        super::save(&project, project_dir)?;
        println!("{} — html, new", written.id);
    } else {
        println!("{} — rewritten", written.id);
    }
    println!("  {} ({} bytes)", written.path, html.len());
    Ok(())
}

/// Prints the page called `id` as it is on disk: the `page_read` MCP tool.
pub(crate) fn read(project_dir: &Path, id: &str) -> Result<()> {
    let project = super::open(project_dir)?;
    let html = match read_page(&project, project_dir, &AssetId::new(id)) {
        Ok(html) => html,
        // The file's own error stays the cause under a context, as the CLI
        // words every other read.
        Err(PageReadError::Unreadable { id, path, source }) => {
            return Err(source).with_context(|| format!("reading the page `{id}` at {path}"));
        }
        Err(refused) => bail!(refused),
    };
    print!("{html}");
    Ok(())
}
