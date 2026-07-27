//! `scorsese new`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};

/// Lays out an empty project directory. The name falls back to the directory's
/// own stem, because `teaser.scor` holding a project called `teaser` is what
/// anyone omitting the flag meant.
pub fn run(directory: &Path, name: Option<String>, fps: Fps) -> Result<()> {
    let name = name.unwrap_or_else(|| {
        directory
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("untitled")
            .to_owned()
    });

    Project::create(directory, &name, fps)
        .with_context(|| format!("creating a project in {}", directory.display()))?;

    println!(
        "Created project \"{name}\" at {fps} fps in {}",
        directory.display()
    );
    println!("  project.json, assets/, generated/, cache/");
    Ok(())
}
