//! Walking the document: its tracks and clips, the timeline's and every
//! group's.
//!
//! **One walk, one answer.** A group's members sit on tracks inside its asset,
//! so "every clip" written by hand has to remember the assets table — and each
//! copy that forgot answered "there is no clip `x`" for a member another tool
//! edited fine (#1103). Anything that looks a clip up by id, or visits every
//! track, comes through here.

use super::Project;
use crate::asset::AssetId;
use crate::timeline::{Clip, ClipId, Track};

impl Project {
    /// Every clip on the project's own timeline, paired with the track holding
    /// it — **not** the members of a group, which sit on the group's tracks
    /// inside its asset. See [`Project::every_clip`] for those too.
    pub fn clips(&self) -> impl Iterator<Item = (&Track, &Clip)> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter().map(move |clip| (track, clip)))
    }

    /// Every clip in the **document**: the timeline's, then the members of
    /// every group in the assets table.
    ///
    /// The question to ask whenever an id or a reference is at stake. Clip ids
    /// are unique across the whole document, groups included — arrows name
    /// clips by id — so a new id has to be free here and not only on the
    /// timeline; and an asset a group's member shows is in use, however
    /// deeply it is nested, because nesting is by reference: every group sits
    /// in the one assets table, so walking each once reaches every member.
    pub fn every_clip(&self) -> impl Iterator<Item = (&Track, &Clip)> {
        self.every_track()
            .flat_map(|track| track.clips.iter().map(move |clip| (track, clip)))
    }

    /// Every track in the **document**: the timeline's, then each group's.
    ///
    /// Track ids are one namespace for the whole document, as clip ids are, so
    /// this is the list a new track's id has to be free in.
    pub fn every_track(&self) -> impl Iterator<Item = &Track> {
        let lanes = self
            .assets
            .iter()
            .filter_map(|asset| asset.group.as_ref())
            .flat_map(|group| &group.tracks);
        self.tracks.iter().chain(lanes)
    }

    /// [`Project::every_track`], to change: the timeline's, then each group's.
    pub fn every_track_mut(&mut self) -> impl Iterator<Item = &mut Track> {
        let lanes = self
            .assets
            .iter_mut()
            .filter_map(|asset| asset.group.as_mut())
            .flat_map(|group| &mut group.tracks);
        self.tracks.iter_mut().chain(lanes)
    }

    /// The clip called `id`, on the timeline or among a group's members —
    /// clip ids are one namespace for the whole document, so there is only
    /// ever one.
    pub fn clip(&self, id: &ClipId) -> Option<&Clip> {
        self.every_clip()
            .map(|(_, clip)| clip)
            .find(|clip| &clip.id == id)
    }

    /// [`Project::clip`], to change.
    pub fn clip_mut(&mut self, id: &ClipId) -> Option<&mut Clip> {
        self.every_track_mut()
            .flat_map(|track| track.clips.iter_mut())
            .find(|clip| &clip.id == id)
    }

    /// The group whose tracks hold clip `id`, or `None` for a clip on the
    /// timeline itself (or no clip at all) — what a refusal names when an edit
    /// only the timeline can take is asked of a member.
    pub fn group_holding(&self, id: &ClipId) -> Option<&AssetId> {
        self.assets.iter().find_map(|asset| {
            let group = asset.group.as_ref()?;
            group
                .clips()
                .any(|(_, clip)| &clip.id == id)
                .then_some(&asset.id)
        })
    }
}
