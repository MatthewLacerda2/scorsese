//! `scorsese level`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_render::audio::reference::Reference;
use scorsese_render::{Tools, audio::measure, say};

/// Measures `file`, and prints how it came out — optionally against another.
///
/// The comparison is the reason this is a command rather than a line of the
/// render report. A number on its own is hard to judge: −14 dBFS is right for
/// one piece and four decibels down on another. A *difference* is not, and the
/// thing to compare against is almost always a file — the previous bake, the
/// version this was meant to replace, the render that sounded right.
///
/// It takes a path rather than an asset id on purpose. What is measured here is
/// a finished file, and a finished file is as likely to be a delivered `.mp4`
/// sitting outside the project as a bake inside it.
pub(crate) fn run(file: &Path, against: Option<&Path>, like: Option<&Reference>) -> Result<()> {
    let tools = Tools::discover()?;
    let profile = measure(&tools, file).with_context(|| format!("measuring {}", file.display()))?;

    println!("{}", say::headline(&say::name(file), &profile));
    for row in say::sections(&profile) {
        println!("  {row}");
    }
    if let Some(reference) = like {
        println!();
        super::synth::likeness(&profile, reference, "");
    }

    let Some(other) = against else {
        return Ok(());
    };
    let previous =
        measure(&tools, other).with_context(|| format!("measuring {}", other.display()))?;
    println!();
    println!("{}  vs  {}", say::name(file), say::name(other));
    for row in say::comparison(&profile, &previous) {
        println!("  {row}");
    }
    Ok(())
}
