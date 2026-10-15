//! `scorsese hear`
//!
//! A waveform of a sound file, written out as a PNG.
//!
//! The audio counterpart of `look`, and a command as well as an MCP tool for
//! the same reason that one is: the capability must not be MCP-only. A person
//! who has just generated six narration lines should be able to see all six
//! without a client in the loop, and the PNG is a file they can keep.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use scorsese_render::audio::waveform;
use scorsese_render::{Tools, frames};

/// Draws the waveform of `file` and writes it to `out`.
pub(crate) fn run(file: &Path, out: Option<PathBuf>) -> Result<()> {
    let tools = Tools::discover()?;
    let wave = waveform(&tools, file).with_context(|| format!("hearing {}", file.display()))?;

    let path = out.unwrap_or_else(|| default_out(file));
    frames::write_png(&tools, &path, &wave.image)
        .with_context(|| format!("writing {}", path.display()))?;

    println!(
        "Wrote {} — {} ({:.2}s), {}",
        path.display(),
        file.display(),
        wave.seconds,
        level(&wave.findings)
    );
    // Said on its own line rather than folded into the level, because these are
    // the findings somebody is looking for and a sentence they scan past is a
    // sentence that did not help.
    for note in wave.findings.says() {
        println!("  {note}");
    }
    Ok(())
}

/// How loud it got, or that it never did.
fn level(findings: &scorsese_render::audio::Findings) -> String {
    match findings.peak_dbfs() {
        None => "SILENT throughout".to_owned(),
        Some(db) => format!("peak {db:.1} dBFS"),
    }
}

/// `vo-01.mp3` becomes `vo-01.wave.png`, beside it.
///
/// Beside the source rather than in the working directory, for the reason
/// `look` gives: hearing six lines in a folder should leave six pictures in
/// that folder, not six files wherever the terminal happened to be.
fn default_out(file: &Path) -> PathBuf {
    let mut name = file.file_stem().unwrap_or_default().to_os_string();
    name.push(".wave.png");
    file.with_file_name(name)
}
