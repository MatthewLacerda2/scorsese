//! `scorsese level`

use std::path::Path;

use anyhow::{Context, Result};
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
pub(crate) fn run(file: &Path, against: Option<&Path>) -> Result<()> {
    let tools = Tools::discover()?;
    let profile = measure(&tools, file).with_context(|| format!("measuring {}", file.display()))?;

    println!("{}", say::headline(&name(file), &profile));
    for row in say::sections(&profile) {
        println!("  {row}");
    }

    let Some(other) = against else {
        return Ok(());
    };
    let previous =
        measure(&tools, other).with_context(|| format!("measuring {}", other.display()))?;
    println!();
    println!("{}  vs  {}", name(file), name(other));
    for row in say::comparison(&profile, &previous) {
        println!("  {row}");
    }
    Ok(())
}

/// How a file is named in the report: its file name, not its whole path.
///
/// The two files in a comparison usually sit in the same directory and differ
/// in one word, and printing two long paths that agree for sixty characters
/// buries the word that differs.
fn name(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
