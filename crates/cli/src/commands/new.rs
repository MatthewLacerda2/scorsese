//! `scorsese new`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::Project;

pub fn run(directory: &Path, name: Option<String>) -> Result<()> {
    let name = name.unwrap_or_else(|| {
        directory
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("untitled")
            .to_owned()
    });

    Project::create(directory, &name)
        .with_context(|| format!("creating a project in {}", directory.display()))?;

    println!("Created project \"{name}\" in {}", directory.display());
    println!("  project.json, assets/, generated/, cache/");
    Ok(())
}
