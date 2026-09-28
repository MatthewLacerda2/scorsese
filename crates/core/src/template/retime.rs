//! A template saved on one frame grid, brought onto another.
//!
//! A template carries the `timeline_fps` it was saved on, so every time in it
//! means seconds on that grid. Onto a project on another grid each time is
//! conformed ([`Fps::conform`]: the nearest frame, nothing invented), and a
//! clip's end is conformed rather than its length, so clips that touched still
//! touch.

use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::ClipId;

/// Every time in `template` moved onto `onto`, or the clip that would be
/// shorter than a frame there.
pub(super) fn retime(template: &mut Project, onto: Fps) -> Result<(), ClipId> {
    let from = template.timeline_fps;
    if from == onto {
        return Ok(());
    }
    let conform = |frames: Frames| onto.conform(frames, from);
    for clip in template
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
    {
        let start = conform(clip.start);
        let end = conform(clip.end());
        if end <= start {
            return Err(clip.id.clone());
        }
        clip.start = start;
        clip.duration = Frames(end.get() - start.get());
        clip.source_in = conform(clip.source_in);
        for key in clip
            .keyframes
            .iter_mut()
            .flat_map(|track| &mut track.keyframes)
        {
            key.t = conform(key.t);
        }
    }
    template.timeline_fps = onto;
    Ok(())
}
