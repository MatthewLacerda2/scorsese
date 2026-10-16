//! Words the band had no room for, and clips that run off the frame (#1096).
//!
//! Both are [`Layout`]'s own findings, read at instants across the timeline
//! and said once per clip, so the checkup and `describe at` cannot word them
//! differently. **Always a warning**: a cut title still renders, and a clip
//! half off the frame is often a slide-in somebody meant.

use std::path::Path;

use scorsese_core::{Frames, Project};

use crate::Overset;

use super::overlap::reference;
use crate::describe::Moment;
use crate::layout::{Layout, Overhang, cut_off};
use crate::plan::{FrameRange, Plan};

/// Every clip whose words were cut or whose content runs off the frame,
/// already worded, in the order the timeline first shows each.
///
/// Each stretch of the plan is read at its **middle**, not its start: a
/// slide-in is moving at the start of its clip and settled by the middle, and
/// a clip still off the frame there is off it for a reason worth a word. No
/// ffmpeg, and a stretch [`Layout`] cannot answer for is silent — whatever made
/// it unanswerable is a finding of its own in the same report.
pub(super) fn off_frame(project: &Project, project_dir: &Path) -> Vec<String> {
    let fps = project.timeline_fps;
    let Ok(plan) = Plan::build(project, fps, FrameRange::ALL) else {
        return Vec::new();
    };
    let mut cut: Vec<(String, Overset)> = Vec::new();
    let mut leaving: Vec<(String, Overhang, Frames)> = Vec::new();
    for segment in plan.segments() {
        let middle = segment.start + Frames(segment.duration.get() / 2);
        let Ok(layout) = Layout::at(project, project_dir, reference(project), middle) else {
            continue;
        };
        for placement in &layout.placed {
            // The same at every instant: the words are set once per clip.
            if let Some(overset) = placement.overset
                && !cut.iter().any(|(clip, _)| *clip == placement.clip)
            {
                cut.push((placement.clip.clone(), overset));
            }
            let Some(overhang) = placement.overhang else {
                continue;
            };
            match leaving
                .iter_mut()
                .find(|(clip, ..)| *clip == placement.clip)
            {
                Some(worst) if furthest(&overhang) > furthest(&worst.1) => {
                    (worst.1, worst.2) = (overhang, middle);
                }
                Some(_) => {}
                None => leaving.push((placement.clip.clone(), overhang, middle)),
            }
        }
    }
    let cut = cut.into_iter().map(|(clip, overset)| {
        format!(
            "text clip `{clip}` {} — widen `max_width`, lower `size` or shorten the text",
            cut_off(&overset)
        )
    });
    let leaving = leaving.into_iter().map(|(clip, overhang, at)| {
        let at = Moment::at(at, fps);
        format!(
            "clip `{clip}` {overhang} at {:.2}s (f{}) — fine if it is meant to (a slide-in, \
             a pan), otherwise move or shrink it",
            at.seconds,
            at.frame.get()
        )
    });
    cut.chain(leaving).collect()
}

/// The deepest any one edge is overrun, which is what makes one instant worse
/// than another.
fn furthest(overhang: &Overhang) -> f64 {
    [overhang.left, overhang.right, overhang.top, overhang.bottom]
        .into_iter()
        .fold(0.0, f64::max)
}
