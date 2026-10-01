//! What a copied clip or group names, renamed to what it is called now.
//!
//! A template's clips name other things by id — the asset a clip shows, the
//! arrow it travels along, and inside a group the members' own ids and the
//! group's lanes. Inserted, each of those may have been given another id, so
//! every reference is renamed with it, or the copy would point at the
//! original, or at nothing.

use std::collections::BTreeMap;

use super::ids::free;
use crate::asset::AssetId;
use crate::group::Group;
use crate::project::Project;
use crate::timeline::{Clip, ClipId, TrackId};

/// The ids the template's clips and assets have in the project.
pub(super) struct Renames {
    /// Each clip's, the members of its groups included.
    pub(super) clips: BTreeMap<ClipId, ClipId>,
    /// Each asset's — its copy's, or the project's own for a file it had.
    pub(super) assets: BTreeMap<AssetId, AssetId>,
}

impl Renames {
    /// `clip` under the names it has here: its own id, the asset it shows,
    /// the arrow it travels along — the copy of that arrow, as an arrow's
    /// attached ends follow the copies of the clips they name — and the clip
    /// it is seen through, its matte, which is a clip of the template too.
    pub(super) fn clip(&self, clip: &Clip) -> Clip {
        let mut copy = clip.clone();
        if let Some(now) = self.clips.get(&clip.id) {
            copy.id = now.clone();
        }
        if let Some(now) = self.assets.get(&clip.asset) {
            copy.asset = now.clone();
        }
        if let Some(follow) = copy.follow.as_mut()
            && let Some(now) = self.clips.get(&follow.clip)
        {
            follow.clip = now.clone();
        }
        if let Some(matte) = copy.matte.as_mut()
            && let Some(now) = self.clips.get(&matte.clip)
        {
            matte.clip = now.clone();
        }
        copy
    }

    /// A copied group's members renamed, and its lanes given ids free in
    /// `proposed` — track ids are one namespace for the whole document, as
    /// clip ids are, so a lane keeps its id only where nothing else has it.
    pub(super) fn group(&self, group: &mut Group, proposed: &Project) {
        let limit = proposed.every_track().count() + group.tracks.len();
        let mut given: Vec<TrackId> = Vec::new();
        for track in &mut group.tracks {
            let id = free(track.id.as_str(), limit, |candidate| {
                proposed.every_track().any(|t| t.id.as_str() == candidate)
                    || given.iter().any(|t| t.as_str() == candidate)
            });
            track.id = TrackId::new(id);
            given.push(track.id.clone());
            for clip in &mut track.clips {
                *clip = self.clip(clip);
            }
        }
    }
}
