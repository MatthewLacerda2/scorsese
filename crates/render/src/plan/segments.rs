//! Cutting the timeline where the set of visible clips changes.
//!
//! With one video track a segment was simply a clip or a hole. With several,
//! the useful unit is a stretch over which **nothing changes**: the same clips
//! are visible on the same tracks throughout, so one decoder per layer covers
//! the whole stretch and the compositor is handed the same shaped stack every
//! frame.
//!
//! Those stretches fall out of the clip boundaries. Every clip start and every
//! clip end, on any track, is a point where the visible set can change; between
//! two consecutive such points it cannot.

use scorsese_core::{Asset, AssetKind, Clip, Frames, Project, Track, TrackKind};

use super::{PlanError, Segment, Shot};

/// Every video track carrying clips, in project order — **first at the bottom**.
pub fn video_tracks(project: &Project) -> Vec<&Track> {
    project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video && !track.clips.is_empty())
        .collect()
}

/// The frame just past the last clip on any of these tracks.
pub fn timeline_end(tracks: &[&Track]) -> Frames {
    tracks
        .iter()
        .flat_map(|track| track.clips.iter().map(Clip::end))
        .max()
        .unwrap_or(Frames::ZERO)
}

/// Splits `start..end` at every clip boundary and gathers what is visible
/// through each piece.
pub fn build<'a>(
    project: &'a Project,
    tracks: &[&'a Track],
    start: Frames,
    end: Frames,
) -> Result<Vec<Segment<'a>>, PlanError> {
    let mut segments = Vec::new();
    for pair in cuts(tracks, start, end).windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let mut layers = Vec::new();
        for track in tracks {
            // A track's clips never overlap — validation guarantees it — so at
            // most one is visible here. A track with a hole contributes **no
            // layer**, rather than a black one that would paint over the tracks
            // below it.
            if let Some(clip) = track.clips.iter().find(|clip| covers(clip, from)) {
                layers.push(Shot {
                    clip,
                    asset: renderable_asset(project, clip)?,
                    source_in: clip.source_in + Frames(from.get() - clip.start.get()),
                });
            }
        }
        segments.push(Segment {
            start: from,
            duration: Frames(to.get() - from.get()),
            layers,
        });
    }
    Ok(segments)
}

/// The frames at which the visible set can change, in order, `start` and `end`
/// included.
fn cuts(tracks: &[&Track], start: Frames, end: Frames) -> Vec<Frames> {
    let mut cuts = vec![start, end];
    let inside = |at: Frames| at > start && at < end;
    for track in tracks {
        for clip in &track.clips {
            for boundary in [clip.start, clip.end()] {
                if inside(boundary) {
                    cuts.push(boundary);
                }
            }
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

fn covers(clip: &Clip, at: Frames) -> bool {
    clip.start <= at && at < clip.end()
}

/// The asset a clip shows, if it is something this pipeline can decode today.
fn renderable_asset<'a>(project: &'a Project, clip: &Clip) -> Result<&'a Asset, PlanError> {
    let asset = project
        .asset(&clip.asset)
        .ok_or_else(|| PlanError::UnknownAsset {
            clip: clip.id.to_string(),
            asset: clip.asset.to_string(),
        })?;
    if asset.needs_generation() {
        return Err(PlanError::NotGenerated {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    if asset.kind == AssetKind::Text {
        return Err(PlanError::NeedsCompositor {
            clip: clip.id.to_string(),
            kind: asset.kind,
        });
    }
    if asset.path.is_none() {
        return Err(PlanError::NoMedia {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    Ok(asset)
}
