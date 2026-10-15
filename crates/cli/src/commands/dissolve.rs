//! `scorsese dissolve`

use std::path::Path;

use anyhow::Result;
use scorsese_core::{ClipId, Frames};
use scorsese_render::{Placed, dissolve};

/// Dissolves one shot into the next, writing ordinary opacity keyframes.
///
/// The incoming clip is pulled back over the outgoing one and moved to a track
/// above, because two clips on one track may not overlap and a crossover needs
/// them to. What that rearranges is printed rather than left to be discovered.
pub(crate) fn run(project_dir: &Path, from: &str, to: &str, seconds: f64) -> Result<()> {
    let mut project = super::open(project_dir)?;

    let duration = Frames::span(seconds, project.timeline_fps.as_f64());
    let placed = dissolve(&mut project, &ClipId::new(from), &ClipId::new(to), duration)?;

    super::save(&project, project_dir)?;
    describe(from, to, duration, &placed);
    Ok(())
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
