//! `scorsese describe`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::Fps;
use scorsese_render::{Cue, FrameRange, Plan, Sizing, described};

/// Prints what the timeline contains, without rendering any of it.
///
/// The cheapest review there is: no ffmpeg, no encode, no money. It reads the
/// document, sequences it exactly as a render would, and says what would come
/// out — which makes it as useful before a render as after one.
///
/// `at` adds *where*: the rectangle each clip's content covers at an instant.
/// It belongs to this verb rather than one of its own because it is the same
/// question — what is in this frame — asked from the other side, and it is the
/// answer that used to cost a still and a pair of eyes. It names the word each
/// narration is saying there too, as `project_describe` does (#1289).
pub(crate) fn run(
    project_dir: &Path,
    fps: Option<Fps>,
    range: Option<FrameRange>,
    at: &[Cue],
    resolution: Option<Sizing>,
) -> Result<()> {
    let project = super::open(project_dir)?;
    let fps = fps.unwrap_or(project.timeline_fps);
    let plan = Plan::build(&project, fps, range.unwrap_or(FrameRange::ALL))
        .context("sequencing the timeline")?;

    // Each instant on the **timeline's** grid, whatever `--fps` a render
    // would be conformed to: the document is authored on it, and `2.5s` means
    // the same instant of the edit however the file is delivered.
    let at: Vec<_> = at
        .iter()
        .map(|cue| cue.timeline_frame(project.timeline_fps))
        .collect();
    // The answer `project_describe` gives, assembled in one place: the
    // script, the cut and its notes, and for each instant where everything
    // lands — against the project's own frame unless `resolution` names
    // another — and which word each narration is saying.
    let described = described(&project, project_dir, &plan, &at, resolution)
        .context("finding what is on screen")?;
    print!("{described}");
    Ok(())
}
