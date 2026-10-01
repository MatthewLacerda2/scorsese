//! A template saved on one frame grid, brought onto another.
//!
//! A template carries the `timeline_fps` it was saved on, so every time in it
//! means seconds on that grid. Onto a project on another grid each time is
//! conformed ([`Fps::conform`]: the nearest frame, nothing invented), and a
//! clip's end is conformed rather than its length, so clips that touched still
//! touch.

use std::collections::BTreeMap;

use crate::asset::AssetId;
use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::{ClipId, Track};

/// Every time in `template` moved onto `onto`, or the clip that would be
/// shorter than a frame there.
///
/// A group's members are conformed too, since they count in frames of the same
/// grid — and then every clip of a group is kept inside it, because the
/// group's length is derived from its members and rounding can leave a clip
/// that showed all of it a frame past the new end.
pub(super) fn retime(template: &mut Project, onto: Fps) -> Result<(), ClipId> {
    let from = template.timeline_fps;
    if from == onto {
        return Ok(());
    }
    let groups = template
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut());
    for group in groups {
        conform(&mut group.tracks, from, onto)?;
    }
    conform(&mut template.tracks, from, onto)?;
    let lengths: BTreeMap<AssetId, Frames> = template
        .assets
        .iter()
        .filter_map(|asset| Some((asset.id.clone(), asset.group.as_ref()?.length())))
        .collect();
    let lanes = template
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut())
        .flat_map(|group| &mut group.tracks)
        .chain(&mut template.tracks);
    for clip in lanes.flat_map(|track| &mut track.clips) {
        let Some(length) = lengths.get(&clip.asset) else {
            continue;
        };
        let room = length.get().saturating_sub(clip.source_in.get());
        if room == 0 {
            return Err(clip.id.clone());
        }
        clip.duration = Frames(clip.duration.get().min(room));
    }
    template.timeline_fps = onto;
    Ok(())
}

/// Every clip on `tracks` moved from the `from` grid onto `onto`.
fn conform(tracks: &mut [Track], from: Fps, onto: Fps) -> Result<(), ClipId> {
    let conform = |frames: Frames| onto.conform(frames, from);
    for clip in tracks.iter_mut().flat_map(|track| &mut track.clips) {
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
    Ok(())
}
