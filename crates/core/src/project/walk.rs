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
use crate::timeline::{Clip, ClipId, Track, TrackId};

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

    /// [`Project::every_clip`], to change.
    pub fn every_clip_mut(&mut self) -> impl Iterator<Item = &mut Clip> {
        self.every_track_mut()
            .flat_map(|track| track.clips.iter_mut())
    }

    /// Every clip in the document that shows asset `id`, a group's members
    /// included — what "is this asset in use?" asks of the timeline, however
    /// deeply a clip of it is nested.
    ///
    /// Only clips: another asset naming `id` (a generated image used as a
    /// reference) is a use too, and the question that counts it asks for it.
    pub fn clips_of<'a>(&'a self, id: &'a AssetId) -> impl Iterator<Item = &'a Clip> {
        self.every_clip()
            .map(|(_, clip)| clip)
            .filter(move |clip| &clip.asset == id)
    }

    /// Every stack of tracks in the document — the timeline's, then each
    /// group's — with the group that owns it, `None` for the timeline.
    ///
    /// A stack is what a clip composites over and an arrow may reach: the
    /// tracks beside it. The question to ask before "is this track in a
    /// group?" or "which clips sit beside this one?".
    pub fn stacks(&self) -> impl Iterator<Item = (Option<&AssetId>, &[Track])> {
        let groups = self.assets.iter().filter_map(|asset| {
            let group = asset.group.as_ref()?;
            Some((Some(&asset.id), group.tracks.as_slice()))
        });
        std::iter::once((None, self.tracks.as_slice())).chain(groups)
    }

    /// [`Project::stacks`], to change — the lists themselves, so a lane can
    /// be added or taken out of the stack it belongs to.
    pub fn stacks_mut(&mut self) -> impl Iterator<Item = &mut Vec<Track>> {
        let groups = self
            .assets
            .iter_mut()
            .filter_map(|asset| asset.group.as_mut())
            .map(|group| &mut group.tracks);
        std::iter::once(&mut self.tracks).chain(groups)
    }

    /// The lowest `{prefix}{n}`, counting from `first`, that no track in the
    /// document is called: track ids are one namespace, a group's lanes
    /// included, so a new lane is named against all of them.
    pub fn free_track_id(&self, prefix: &str, first: usize) -> TrackId {
        let taken = |candidate: &str| {
            self.every_track()
                .any(|track| track.id.as_str() == candidate)
        };
        TrackId::new(crate::ids::numbered(
            prefix,
            first,
            self.every_track().count(),
            taken,
        ))
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
        self.every_clip_mut().find(|clip| &clip.id == id)
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
