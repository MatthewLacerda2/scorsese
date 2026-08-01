//! `scorsese dissolve`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{ClipId, Frames, Project};
use scorsese_render::{Placed, dissolve};

/// Dissolves one shot into the next, writing ordinary opacity keyframes.
///
/// The incoming clip is pulled back over the outgoing one and moved to a track
/// above, because two clips on one track may not overlap and a crossover needs
/// them to. What that rearranges is printed rather than left to be discovered.
pub(crate) fn run(project_dir: &Path, from: &str, to: &str, seconds: f64) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let duration = frames(seconds, project.timeline_fps.as_f64());
    let placed = dissolve(&mut project, &ClipId::new(from), &ClipId::new(to), duration)?;

    project.save(project_dir).context("saving the project")?;
    describe(from, to, duration, &placed);
    Ok(())
}

/// Seconds on the project's own grid, at least one frame when anything was
/// asked for — a crossover rounded to zero frames is a cut, not a dissolve.
fn frames(seconds: f64, fps: f64) -> Frames {
    if seconds <= 0.0 {
        return Frames::ZERO;
    }
    Frames(((seconds * fps).round() as u64).max(1))
}

/// Says what happened, including the part nobody asked for in so many words.
fn describe(from: &str, to: &str, duration: Frames, placed: &Placed) {
    println!(
        "{from} dissolves into {to} over {} frames — ordinary opacity keyframes, yours to edit",
        duration.get()
    );
    let track = &placed.track;
    if placed.track_created {
        println!("`{to}` moved to a new track `{track}` above it, so the two can overlap");
    } else {
        println!("`{to}` moved up to track `{track}`, so the two can overlap");
    }
    println!(
        "`{to}` now starts {} frames earlier than it did",
        placed.pulled_back
    );
}
