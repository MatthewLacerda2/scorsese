//! Putting the moved clips back, each where there is room for it.

use std::collections::BTreeSet;

use super::lay::Move;
use crate::authoring::numbered;
use crate::project::Project;
use crate::timeline::{Clip, ClipId, Track, TrackId};

/// Takes every moved clip off its track and puts it back at its new place:
/// on its own track when that has room, else on the next track of its kind
/// above that has, else on one made for it directly above its own.
///
/// Everything moved is lifted first, so the clips being laid out never block
/// each other's old places — only their new ones, and anything not moved.
/// Hands back the clips that changed track.
pub(super) fn apply(project: &mut Project, moves: &[Move]) -> Vec<(ClipId, TrackId, bool)> {
    let moving: BTreeSet<&ClipId> = moves.iter().map(|m| &m.clip).collect();
    let mut lifted: Vec<(TrackId, Clip)> = Vec::new();
    for track in &mut project.tracks {
        let (gone, kept) = std::mem::take(&mut track.clips)
            .into_iter()
            .partition(|clip| moving.contains(&clip.id));
        track.clips = kept;
        lifted.extend(gone.into_iter().map(|clip: Clip| (track.id.clone(), clip)));
    }

    let mut rearranged = Vec::new();
    for change in moves {
        let Some(at) = lifted.iter().position(|(_, clip)| clip.id == change.clip) else {
            continue;
        };
        let (home, mut clip) = lifted.swap_remove(at);
        clip.start = change.start;
        clip.duration = change.duration;
        let (track, made) = room(project, &home, &clip);
        if track != home {
            rearranged.push((clip.id.clone(), track.clone(), made));
        }
        let track = project
            .tracks
            .iter_mut()
            .find(|candidate| candidate.id == track)
            .expect("room hands back a track it found or made");
        track.clips.push(clip);
        track.clips.sort_by_key(|clip| clip.start);
    }
    rearranged
}

/// The track `clip` goes on, starting from `home`, and whether it was made.
fn room(project: &mut Project, home: &TrackId, clip: &Clip) -> (TrackId, bool) {
    let index = project
        .tracks
        .iter()
        .position(|track| &track.id == home)
        .expect("a lifted clip's track is still there");
    let kind = project.tracks[index].kind;
    let free = project.tracks[index..]
        .iter()
        .filter(|track| track.kind == kind)
        .find(|track| !track.clips.iter().any(|other| other.overlaps(clip)));
    if let Some(track) = free {
        return (track.id.clone(), false);
    }
    // Directly above its own track rather than on top of everything, so an
    // unrelated overlay stays above both — the place `dissolve` chooses too.
    let id = numbered(project, kind);
    project
        .tracks
        .insert(index + 1, Track::new(id.clone(), kind));
    (id, true)
}
