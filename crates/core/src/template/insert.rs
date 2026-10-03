//! Copying a template into a project at a chosen time.

use std::collections::BTreeMap;

use super::ids::free;
use super::rename::Renames;
use super::retime::retime;
use crate::asset::AssetId;
use crate::authoring::numbered;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId, Track, TrackId, TrackKind};
use crate::validate::ValidationErrors;

/// What an insertion wrote, once the document accepted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inserted {
    /// Each clip written, by the id it has now, and the track it went on. In
    /// the template's order.
    pub clips: Vec<(ClipId, TrackId)>,
    /// The tracks made for it, in the order they were appended — on top.
    pub new_tracks: Vec<TrackId>,
    /// The assets added to the table, by the ids they have now.
    pub added: Vec<AssetId>,
    /// The template's files the project already had, by the project's id for
    /// each: referenced, not added twice.
    pub reused: Vec<AssetId>,
    /// Where the inserted stretch begins on the timeline.
    pub start: Frames,
    /// Where it ends: the end of its last clip.
    pub end: Frames,
}

/// Why nothing was inserted. Every one leaves the project untouched.
#[derive(Debug, thiserror::Error)]
pub enum InsertError {
    /// The template holds no clip at all.
    #[error("the template has no clips in it")]
    Empty,
    /// On this project's frame grid a clip would last less than one frame.
    #[error(
        "on this project's frame rate, the template's clip `{0}` would be shorter than a frame"
    )]
    Vanishes(ClipId),
    /// The result was not a document that loads.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Copy `template` into `project` so that its first clip starts at `at`.
///
/// Where each of its tracks lands, and why, is the module's doc
/// ([`crate::template`]); what came of it is the [`Inserted`] returned.
pub fn insert(
    project: &mut Project,
    template: &Project,
    at: Frames,
) -> Result<Inserted, InsertError> {
    let mut template = template.clone();
    retime(&mut template, project.timeline_fps).map_err(InsertError::Vanishes)?;
    let opens = template
        .clips()
        .map(|(_, clip)| clip.start)
        .min()
        .ok_or(InsertError::Empty)?;

    let mut proposed = project.clone();
    let mut done = Inserted {
        clips: Vec::new(),
        new_tracks: Vec::new(),
        added: Vec::new(),
        reused: Vec::new(),
        start: at,
        end: at,
    };
    let renames = copy_assets(
        &mut proposed,
        &template,
        clip_ids(project, &template),
        &mut done,
    );

    for kind in [TrackKind::Video, TrackKind::Audio] {
        let lanes: Vec<usize> = lanes_of(&proposed, kind);
        let mut spilled = false;
        for (nth, track) in template
            .tracks
            .iter()
            .filter(|t| t.kind == kind)
            .enumerate()
        {
            if track.clips.is_empty() {
                continue;
            }
            let clips: Vec<Clip> = track
                .clips
                .iter()
                .map(|clip| Clip {
                    start: Frames(at.get() + clip.start.get() - opens.get()),
                    ..renames.clip(clip)
                })
                .collect();
            let lane = match lanes.get(nth) {
                Some(&lane) if !spilled && fits(&proposed.tracks[lane], &clips) => lane,
                _ => {
                    spilled = true;
                    let taken = |id: &str| proposed.every_track().any(|t| t.id.as_str() == id);
                    let id = if taken(track.id.as_str()) {
                        numbered(&proposed, kind)
                    } else {
                        track.id.clone()
                    };
                    let mut lane = Track::new(id.clone(), kind);
                    lane.name.clone_from(&track.name);
                    lane.note.clone_from(&track.note);
                    proposed.tracks.push(lane);
                    done.new_tracks.push(id);
                    proposed.tracks.len() - 1
                }
            };
            let lane = &mut proposed.tracks[lane];
            for clip in clips {
                done.end = done.end.max(clip.end());
                done.clips.push((clip.id.clone(), lane.id.clone()));
                lane.clips.push(clip);
            }
            lane.clips.sort_by_key(|clip| clip.start);
        }
    }
    proposed.validate()?;
    *project = proposed;
    Ok(done)
}

/// The id each of the template's clips gets, its groups' members included:
/// its own where free in the whole document.
fn clip_ids(project: &Project, template: &Project) -> BTreeMap<ClipId, ClipId> {
    let mut given: BTreeMap<ClipId, ClipId> = BTreeMap::new();
    let limit = project.every_clip().count() + template.every_clip().count();
    for (_, clip) in template.every_clip() {
        let id = free(clip.id.as_str(), limit, |candidate| {
            project
                .every_clip()
                .any(|(_, c)| c.id.as_str() == candidate)
                || given.values().any(|c| c.as_str() == candidate)
        });
        given.insert(clip.id.clone(), ClipId::new(id));
    }
    given
}

/// Add the template's assets to `proposed`, and answer what each clip and
/// asset is called there.
///
/// A file the project already has — the same `sha256` — is the project's own
/// asset; everything else is copied under its own id where free, with the
/// stills its brief names or its sequence plays, the clips its arrow follows
/// and a group's members and lanes renamed to match.
fn copy_assets(
    proposed: &mut Project,
    template: &Project,
    clips: BTreeMap<ClipId, ClipId>,
    done: &mut Inserted,
) -> Renames {
    let mut given: BTreeMap<AssetId, AssetId> = BTreeMap::new();
    let limit = proposed.assets.len() + template.assets.len();
    let mut copies = Vec::new();
    for asset in &template.assets {
        let known = asset.sha256.as_deref().and_then(|hash| {
            proposed
                .assets
                .iter()
                .find(|a| a.sha256.as_deref() == Some(hash))
        });
        if let Some(known) = known {
            done.reused.push(known.id.clone());
            given.insert(asset.id.clone(), known.id.clone());
            continue;
        }
        let id = AssetId::new(free(asset.id.as_str(), limit, |candidate| {
            proposed.assets.iter().any(|a| a.id.as_str() == candidate)
                || given.values().any(|a| a.as_str() == candidate)
        }));
        given.insert(asset.id.clone(), id.clone());
        let mut copy = asset.clone();
        copy.id = id.clone();
        copies.push(copy);
        done.added.push(id);
    }
    let renames = Renames {
        clips,
        assets: given,
    };
    for mut copy in copies {
        let rename = |id: &mut AssetId| {
            if let Some(now) = renames.assets.get(id) {
                *id = now.clone();
            }
        };
        if let Some(brief) = copy.video.as_mut() {
            brief.first_image.iter_mut().for_each(rename);
            brief.last_image.iter_mut().for_each(rename);
            brief.reference_images.iter_mut().for_each(rename);
        }
        if let Some(brief) = copy.image.as_mut() {
            brief.reference_images.iter_mut().for_each(rename);
        }
        if let Some(sequence) = copy.sequence.as_mut() {
            sequence.stills.iter_mut().for_each(rename);
        }
        for attach in super::follows_mut(&mut copy) {
            if let Some(now) = renames.clips.get(&attach.clip) {
                attach.clip = now.clone();
            }
        }
        if let Some(group) = copy.group.as_mut() {
            renames.group(group, proposed);
        }
        proposed.assets.push(copy);
    }
    renames
}

/// The positions of `kind`'s tracks, bottom first.
fn lanes_of(project: &Project, kind: TrackKind) -> Vec<usize> {
    project
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.kind == kind)
        .map(|(at, _)| at)
        .collect()
}

/// Whether `clips` sit on `track` without sharing a frame with what is there.
fn fits(track: &Track, clips: &[Clip]) -> bool {
    clips.iter().all(|new| {
        track
            .clips
            .iter()
            .all(|old| new.end() <= old.start || old.end() <= new.start)
    })
}
