//! `scorsese describe`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::{Description, FrameRange, Note, Plan, unknown_in};

/// Prints what the timeline contains, without rendering any of it.
///
/// The cheapest review there is: no ffmpeg, no encode, no money. It reads the
/// document, sequences it exactly as a render would, and says what would come
/// out — which makes it as useful before a render as after one.
pub fn run(project_dir: &Path, fps: Option<Fps>, range: Option<FrameRange>) -> Result<()> {
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let fps = fps.unwrap_or(project.timeline_fps);
    let plan = Plan::build(&project, fps, range.unwrap_or(FrameRange::ALL))
        .context("sequencing the timeline")?;

    println!("{} — {}", project.name, Description::of(&plan));
    // The same notes a render would carry, for the same reason: a description
    // of a cut whose music was silently trimmed should say so where the cut is
    // being looked at, not only where it is being encoded.
    for note in plan.notes() {
        println!("  note: {note}");
    }
    for unknown in unknown_in(&project) {
        println!("  note: {}", Note::from(unknown));
    }
    Ok(())
}
