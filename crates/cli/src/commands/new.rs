//! `scorsese new`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};

/// Lays out an empty project directory. The name falls back to the directory's
/// own stem, because `teaser.scor` holding a project called `teaser` is what
/// anyone omitting the flag meant.
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
    println!("  project.json, assets/, generated/, recipes/, cache/");
    Ok(())
}
