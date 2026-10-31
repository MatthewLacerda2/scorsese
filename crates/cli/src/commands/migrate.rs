//! `scorsese migrate`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{SCHEMA_VERSION, migrate};
use scorsese_providers::spending::follow_renames;

/// Rewrites the folder's `project.json` at this build's `schema_version`.
///
/// The steps are `scorsese_core::migrate`'s, the same ones the web server
/// runs over every stored project when it starts on a new build — so a local
/// folder and a hosted project are carried forward by one implementation, and
/// this command is only the reading and the printing. An asset id the steps
/// renamed is followed in the spending ledger too, which only a folder has,
/// before the document is written.
pub(crate) fn run(project_dir: &Path) -> Result<()> {
    let migrated =
        migrate::folder_then(project_dir, |renamed| follow_renames(project_dir, renamed))
            .with_context(|| format!("migrating the project in {}", project_dir.display()))?;
    let Some(migrated) = migrated else {
        println!("Already schema_version {SCHEMA_VERSION}; nothing to do");
        return Ok(());
    };
    println!(
        "Migrated {} from schema_version {} to {SCHEMA_VERSION}",
        project_dir.display(),
        migrated.from
    );
    for (old, new) in &migrated.renamed {
        println!("  asset `{old}` is now `{new}`: an asset id is a file name");
    }
    Ok(())
}
