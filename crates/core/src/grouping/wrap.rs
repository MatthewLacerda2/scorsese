//! Wrapping clips already on the timeline into a new group.

use std::collections::BTreeSet;

use super::{every_track, id_limit};
use crate::asset::{Asset, AssetId};
use crate::group::Group;
use crate::project::Project;
use crate::template::ids::free;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId, Track, TrackId, TrackKind};
use crate::validate::ValidationErrors;

/// What to group, and what to call what comes out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grouping {
    /// The clips to wrap, by id — all on the project's own video tracks.
    pub clips: BTreeSet<ClipId>,
    /// The new group asset's id. `None` takes `group`, suffixed until free.
    pub asset: Option<AssetId>,
    /// The id of the clip that shows it. `None` takes `c-` and the asset's id.
    pub clip: Option<ClipId>,
    /// Which track the group clip goes on. `None` is the lowest track any of
    /// the clips came from — see [`group`] for why.
    pub track: Option<TrackId>,
}

/// What a grouping wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grouped {
    /// The new group asset.
    pub asset: AssetId,
    /// The clip showing it.
    pub clip: ClipId,
    /// The track that clip is on.
    pub track: TrackId,
    /// Where the group clip starts: where the earliest wrapped clip did.
    pub start: Frames,
    /// How long it runs: to where the last wrapped clip ended.
    pub duration: Frames,
    /// The group's own tracks, bottom first, each named after the track its
    /// clips came from.
    pub tracks: Vec<TrackId>,
}

/// Why nothing was grouped. Nothing is ever partly grouped either.
#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    /// No clip was named.
    #[error("no clip was named — say which clips to group")]
    Nothing,
    /// An id named no clip on the timeline.
    #[error("no clip on the timeline is called `{clip}` — nothing was grouped")]
    NoSuchClip {
        /// The first id that matched nothing.
        clip: ClipId,
    },
    /// A clip on an audio track. A group is one picture.
    #[error("`{clip}` is on audio track `{track}`, and a group is picture only")]
    NotPicture {
        /// The clip.
        clip: ClipId,
        /// Its audio track.
        track: TrackId,
    },
    /// An id asked for is already taken.
    #[error("`{id}` is already taken in this project — choose another, or leave it out")]
    TakenId {
        /// The id asked for.
        id: String,
    },
    /// The track asked for is not a video track of this project.
    #[error("there is no video track `{track}` to put the group on")]
    NoSuchTrack {
        /// The track asked for.
        track: TrackId,
    },
    /// The result is not a document that loads — an arrow attached across the
    /// new group's edge, or the group clip overlapping something on its track.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Moves the named clips into a new `group` asset and puts one clip of it on
/// the timeline, at the same place and time.
///
/// **The group's time starts at the earliest clip**, so the group clip starts
/// there and runs to where the last one ended, and every member keeps its
/// place relative to the rest. **Its tracks are the tracks the clips came
/// from**, in the same order, so what was drawn over what still is.
///
/// **The clip goes on the lowest of those tracks** unless another is named. The
/// group is one layer, and it has to sit somewhere in the stack: on the lowest,
/// nothing that was under the diagram ends up over it, which is the reading a
/// diagram laid over a background wants. A clip on a track in between, left out
/// of the group, is now drawn over the whole of it — the reply says where the
/// group went, and `track` moves it.
pub fn group(project: &mut Project, grouping: &Grouping) -> Result<Grouped, GroupError> {
    let ids = &grouping.clips;
    if ids.is_empty() {
        return Err(GroupError::Nothing);
    }
    for id in ids {
        let (track, _) = project
            .clips()
            .find(|(_, clip)| &clip.id == id)
            .ok_or_else(|| GroupError::NoSuchClip { clip: id.clone() })?;
        if track.kind != TrackKind::Video {
            return Err(GroupError::NotPicture {
                clip: id.clone(),
                track: track.id.clone(),
            });
        }
    }
    let asset = new_asset_id(project, grouping.asset.as_ref())?;
    let clip = new_clip_id(project, grouping.clip.as_ref(), &asset)?;

    let chosen = || project.clips().filter(|(_, clip)| ids.contains(&clip.id));
    let start = chosen().map(|(_, c)| c.start).min().unwrap_or_default();
    let end = chosen().map(|(_, c)| c.end()).max().unwrap_or_default();

    let mut proposed = project.clone();
    let mut inner = Vec::new();
    let mut lowest = None;
    for (at, track) in proposed.tracks.iter_mut().enumerate() {
        let (moved, kept): (Vec<Clip>, Vec<Clip>) = std::mem::take(&mut track.clips)
            .into_iter()
            .partition(|clip| ids.contains(&clip.id));
        track.clips = kept;
        if moved.is_empty() {
            continue;
        }
        lowest.get_or_insert(at);
        let mut lane = Track::new(TrackId::new(String::new()), TrackKind::Video);
        lane.name.clone_from(&track.name);
        lane.clips = moved
            .into_iter()
            .map(|clip| Clip {
                start: Frames(clip.start.get() - start.get()),
                ..clip
            })
            .collect();
        inner.push((track.id.clone(), lane));
    }
    let tracks = name_lanes(project, &asset, &mut inner);

    let lane = match &grouping.track {
        Some(named) => proposed
            .tracks
            .iter()
            .position(|t| &t.id == named && t.kind == TrackKind::Video)
            .ok_or_else(|| GroupError::NoSuchTrack {
                track: named.clone(),
            })?,
        None => lowest.unwrap_or_default(),
    };
    let duration = Frames(end.get() - start.get());
    let placed = Clip::new(clip.clone(), asset.clone(), start, duration);
    let track = &mut proposed.tracks[lane];
    track.clips.push(placed);
    track.clips.sort_by_key(|clip| clip.start);
    let track = track.id.clone();
    let lanes = inner.into_iter().map(|(_, lane)| lane).collect();
    proposed
        .assets
        .push(Asset::group(asset.clone(), Group::new(lanes)));

    proposed.validate()?;
    *project = proposed;
    Ok(Grouped {
        asset,
        clip,
        track,
        start,
        duration,
        tracks,
    })
}

/// Names each of the group's tracks after the track its clips came from —
/// `diagram-v3` for the clips that were on `v3` — suffixed until free across
/// the whole document.
fn name_lanes(project: &Project, asset: &AssetId, inner: &mut [(TrackId, Track)]) -> Vec<TrackId> {
    let limit = id_limit(project) + inner.len();
    let mut given: Vec<TrackId> = Vec::new();
    for (from, lane) in inner.iter_mut() {
        let wanted = format!("{asset}-{from}");
        let id = TrackId::new(free(&wanted, limit, |candidate| {
            every_track(project).any(|t| t.id.as_str() == candidate)
                || given.iter().any(|g| g.as_str() == candidate)
        }));
        lane.id = id.clone();
        given.push(id);
    }
    given
}

/// The group asset's id: the one asked for if it is free, or `group`
/// suffixed until it is.
fn new_asset_id(project: &Project, wanted: Option<&AssetId>) -> Result<AssetId, GroupError> {
    let taken = |candidate: &str| project.assets.iter().any(|a| a.id.as_str() == candidate);
    match wanted {
        Some(id) if taken(id.as_str()) => Err(GroupError::TakenId { id: id.to_string() }),
        Some(id) => Ok(id.clone()),
        None => Ok(AssetId::new(free("group", id_limit(project), taken))),
    }
}

/// The group clip's id: the one asked for if it is free anywhere in the
/// document, or `c-` and the asset's id suffixed until it is.
fn new_clip_id(
    project: &Project,
    wanted: Option<&ClipId>,
    asset: &AssetId,
) -> Result<ClipId, GroupError> {
    let taken = |candidate: &str| {
        project
            .every_clip()
            .any(|(_, c)| c.id.as_str() == candidate)
    };
    match wanted {
        Some(id) if taken(id.as_str()) => Err(GroupError::TakenId { id: id.to_string() }),
        Some(id) => Ok(id.clone()),
        None => Ok(ClipId::new(free(
            &format!("c-{asset}"),
            id_limit(project),
            taken,
        ))),
    }
}
