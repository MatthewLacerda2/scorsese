//! `scorsese migrate`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{SCHEMA_VERSION, migrate};

/// Rewrites the folder's `project.json` at this build's `schema_version`.
///
/// The steps are `scorsese_core::migrate`'s, the same ones the web server
/// runs over every stored project when it starts on a new build — so a local
/// folder and a hosted project are carried forward by one implementation, and
/// this command is only the reading and the printing.
pub(crate) fn run(project_dir: &Path) -> Result<()> {
    let from = migrate::folder(project_dir)
        .with_context(|| format!("migrating the project in {}", project_dir.display()))?;
    match from {
        Some(from) => println!(
            "Migrated {} from schema_version {from} to {SCHEMA_VERSION}",
            project_dir.display()
        ),
        None => println!("Already schema_version {SCHEMA_VERSION}; nothing to do"),
    }
    Ok(())
}
