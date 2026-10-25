//! What one asset names in its own brief: the one list of it.
//!
//! Pool garbage collection (`pool/gc`) and pool status (`pool/status`) both ask
//! "what does this asset lean on?". Two hand-kept lists drift: gc's forgot a
//! shot's stills and deleted pictures a project still named (#1100). A new
//! field that names an asset is added here, and every caller follows.

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
}
