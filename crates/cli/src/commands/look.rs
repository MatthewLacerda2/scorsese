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
use scorsese_core::words::Words;
use scorsese_render::contact::{self, Look};
use scorsese_render::{Tools, frames};

/// What `scorsese look` was asked: a [`glance`] when no moment is named,
/// otherwise the [`run`] they name, from 0 and five frames unless told.
pub(crate) fn asked(
    file: &Path,
    out: Option<PathBuf>,
    from: Option<f64>,
    to: Option<f64>,
    frames: Option<usize>,
    grid: bool,
) -> Result<()> {
    if from.is_none() && to.is_none() && frames.is_none() {
        return glance(file, out, grid);
    }
    let look = Look {
        from_seconds: from.unwrap_or(0.0),
        to_seconds: to,
        count: frames.unwrap_or(contact::MAX_FRAMES),
        grid,
    };
    run(file, out, &look)
}

/// Takes a sheet of `file` and writes it to `out`.
fn run(file: &Path, out: Option<PathBuf>, look: &Look) -> Result<()> {
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
        contact::counted(sheet.at_seconds.len()),
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

/// Glances at the whole of `file` ([`contact::glance`]) and writes the
/// sheet to `out`, printing the transcript beside the file when it has one.
fn glance(file: &Path, out: Option<PathBuf>, grid: bool) -> Result<()> {
    let tools = Tools::discover()?;
    let words = Words::heard_in(file);
    let glance = contact::glance(&tools, file, words.as_ref(), grid)
        .with_context(|| format!("looking at {}", file.display()))?;
    let sheet = &glance.sheet;

    let path = out.unwrap_or_else(|| default_out(file));
    frames::write_png(&tools, &path, &sheet.image)
        .with_context(|| format!("writing {}", path.display()))?;

    let moments: Vec<String> = sheet
        .at_seconds
        .iter()
        .map(|at| contact::label(*at))
        .collect();
    println!(
        "Wrote {} — {} across the whole of {} ({:.1}s), at {}, blank frames and near-repeats \
         left out{}",
        path.display(),
        contact::counted(sheet.at_seconds.len()),
        file.display(),
        sheet.duration_seconds,
        moments.join(", "),
        if grid {
            ", each ruled in fractions of the source"
        } else {
            ""
        }
    );
    match &glance.transcript {
        Some(lines) => println!("What is said, by when it starts:\n{lines}"),
        None => println!("It has no transcript."),
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
