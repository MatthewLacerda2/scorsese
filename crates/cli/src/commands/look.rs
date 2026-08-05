//! `scorsese look`
//!
//! A contact sheet of a video file, written out as a PNG.
//!
//! Not a project command, and that is the difference from `still`. `still`
//! composites the *edit* at an instant; this decodes a **file** — an imported
//! asset, or footage nobody has imported yet — so the question it answers is
//! "what is in this?" rather than "what does my cut look like?". Neither needs
//! the other and neither replaces it.
//!
//! It exists as a command as well as an MCP tool so the capability is not
//! MCP-only: a person who wants to skim forty clips before importing any of
//! them should be able to.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use scorsese_render::contact::{self, Look};
use scorsese_render::{Tools, frames};

/// Takes a sheet of `file` and writes it to `out`.
pub(crate) fn run(file: &Path, out: Option<PathBuf>, look: &Look) -> Result<()> {
    let tools = Tools::discover()?;
    let sheet = contact::sheet(&tools, file, look)
        .with_context(|| format!("looking at {}", file.display()))?;

    let path = out.unwrap_or_else(|| default_out(file));
    frames::write_png(&tools, &path, &sheet.image)
        .with_context(|| format!("writing {}", path.display()))?;

    let moments: Vec<String> = sheet
        .at_seconds
        .iter()
        .map(|at| contact::label(*at))
        .collect();
    println!(
        "Wrote {} — {} of {} ({:.1}s), at {}{}",
        path.display(),
        counted(sheet.at_seconds.len()),
        file.display(),
        sheet.duration_seconds,
        moments.join(", "),
        if look.grid {
            ", each ruled in fractions of the source"
        } else {
            ""
        }
    );
    // The number a following call starts from, rather than left to be worked
    // out — walking a long file is the expected way to use this, and a step
    // somebody has to compute is a step somebody gets wrong.
    if let Some(next) = sheet.next_from() {
        println!("More to see: --from {next}");
    }
    Ok(())
}

/// `some.mp4` becomes `some.sheet.png`, beside it.
///
/// Beside the source rather than in the working directory, because looking at
/// forty clips in a folder should leave forty sheets in that folder and not
/// forty files wherever the terminal happened to be.
fn default_out(file: &Path) -> PathBuf {
    let mut name = file.file_stem().unwrap_or_default().to_os_string();
    name.push(".sheet.png");
    file.with_file_name(name)
}

/// "1 frame" or "4 frames", because a report that reads wrong reads as a bug.
fn counted(frames: usize) -> String {
    if frames == 1 {
        String::from("1 frame")
    } else {
        format!("{frames} frames")
    }
}
