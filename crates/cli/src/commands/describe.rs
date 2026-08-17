//! `scorsese describe`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::{
    Commentary, Cue, Description, FrameRange, Layout, Note, Plan, Resolution, unknown_in,
};

/// Prints what the timeline contains, without rendering any of it.
///
/// The cheapest review there is: no ffmpeg, no encode, no money. It reads the
/// document, sequences it exactly as a render would, and says what would come
/// out — which makes it as useful before a render as after one.
///
/// `at` adds *where*: the rectangle each clip's content covers at an instant.
/// It belongs to this verb rather than one of its own because it is the same
/// question — what is in this frame — asked from the other side, and it is the
/// answer that used to cost a still and a pair of eyes.
pub(crate) fn run(
    project_dir: &Path,
    fps: Option<Fps>,
    range: Option<FrameRange>,
    at: &[Cue],
    resolution: Resolution,
) -> Result<()> {
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let fps = fps.unwrap_or(project.timeline_fps);
    let plan = Plan::build(&project, fps, range.unwrap_or(FrameRange::ALL))
        .context("sequencing the timeline")?;

    // Before the cut rather than after it. A script is meant to be read before
    // the edit is touched, and a note is the context the shot below it only
    // makes sense in — printing either as a footnote is printing it where the
    // reader has already made up their mind.
    let commentary = Commentary::of(&project);
    if !commentary.is_empty() {
        println!("{commentary}\n");
    }
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
    // Each instant on its own, in the order they were named: a caller checking
    // that a caption still fits at three points in the film is asking three
    // questions, and the answers are only useful side by side.
    for cue in at {
        // The **timeline's** grid, whatever `--fps` a render would be conformed
        // to: the document is authored on it, and `2.5s` means the same instant
        // of the edit however the file is delivered.
        let frame = cue.timeline_frame(project.timeline_fps);
        let layout = Layout::at(&project, project_dir, resolution, frame)
            .with_context(|| format!("finding what is on screen at {cue}"))?;
        println!("\n{layout}");
    }
    Ok(())
}
