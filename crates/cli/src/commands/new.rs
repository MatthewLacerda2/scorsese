//! `scorsese new`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};

/// Lays out an empty project directory. The name falls back to the directory's
/// own stem — a default [`Project::create`] owns, so that `scorsese new` and the
/// MCP `project_new` tool name a project the same way.
///
/// **No stub `script.md`**, and the asymmetry with `recipes/` is the reason.
/// An empty *directory* is unambiguous: it says "things go here" and claims
/// nothing. An empty *file* with `"script": "script.md"` pointing at it is a
/// project claiming to carry a script and carrying nothing — and it disables
/// the one signal that would ever say otherwise, because a `script` naming a
/// file that is not there is a warning precisely so a project that *lost* its
/// brief reports it. A stub makes every project look permanently as though it
/// had one. Absent means nobody has written one yet, which is true of a project
/// a second old.
///
/// What a stub was for — inviting the writing — is bought without the claim by
/// the MCP `script_write` tool, which creates the file and points the document
/// at it in one call. Starting a script is one action either way; only one of
/// the two leaves the document honest in the meantime.
pub(crate) fn run(directory: &Path, name: Option<String>, fps: Fps) -> Result<()> {
    let project = Project::create(directory, name.as_deref(), fps)
        .with_context(|| format!("creating a project in {}", directory.display()))?;

    println!(
        "Created project \"{}\" at {fps} fps in {}",
        project.name,
        directory.display()
    );
    println!("  project.json, assets/, generated/, recipes/, cache/");
    Ok(())
}
