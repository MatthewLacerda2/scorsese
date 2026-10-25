//! What one asset names in its own brief: the one list of it.
//!
//! Collecting and garbage-collecting both ask "what does this
//! asset lean on?", and each used to answer it by hand. Three lists drift: gc's
//! forgot a shot's stills and deleted pictures a project still named (#1100).
//! A new field that names an asset is added here, and every caller follows.

use super::{Asset, AssetId, ImageRequest, VideoRequest};

impl Asset {
    /// Every asset this one names, in field order: a shot's stills, a
    /// generated picture's references, then the stills a sequence plays.
    ///
    /// Not the clips a group holds — those are clips like any other, and
    /// [`crate::Project::every_clip`] is where clips are found.
    pub fn references(&self) -> impl Iterator<Item = &AssetId> {
        let shot = self.video.iter().flat_map(VideoRequest::images);
        let still = self.image.iter().flat_map(ImageRequest::references);
        let played = self.sequence.iter().flat_map(|sequence| &sequence.stills);
        shot.chain(still).chain(played)
    }

    /// [`Self::references`], to edit — the same ids in the same order, for
    /// renaming an asset everywhere it is named.
    pub fn references_mut(&mut self) -> impl Iterator<Item = &mut AssetId> {
        let shot = self.video.iter_mut().flat_map(VideoRequest::images_mut);
        let still = self.image.iter_mut().flat_map(ImageRequest::references_mut);
        let played = self
            .sequence
            .iter_mut()
            .flat_map(|sequence| &mut sequence.stills);
        shot.chain(still).chain(played)
    }
}
