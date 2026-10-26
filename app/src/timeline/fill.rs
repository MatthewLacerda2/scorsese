//! What a made clip is filled with: its picture tiled along it, or its
//! sound's waveform. Resolved once a frame for the assets on the timeline, so
//! the painting that follows only reads.
//!
//! `web/src/editor/timeline/ClipFill.tsx`: footage and stills show the asset's
//! thumbnail (the one the files panel makes and keeps, so nothing is decoded
//! twice), sound shows the levels its file reads as, and anything else the
//! solid hue — which is to say, nothing resolved here.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Context, TextureHandle};
use scorsese_core::{Asset, AssetId, AssetKind};
use scorsese_render::audio::Peaks;

use super::sound::Sounds;
use crate::files::thumbs::{self, Thumbs};
use crate::project::Open;

/// What one asset's clips are filled with.
#[derive(Clone)]
pub(super) enum Fill {
    /// A picture, tiled along the clip.
    Picture(TextureHandle),
    /// A sound's levels.
    Sound(Arc<Peaks>),
}

/// The assets on the timeline that have a fill, by id.
pub(super) type Fills = HashMap<AssetId, Fill>;

/// The workers that make fills, and what they have delivered.
#[derive(Default)]
pub(super) struct Makers {
    thumbs: Thumbs,
    sounds: Sounds,
}

impl std::fmt::Debug for Makers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Makers").finish_non_exhaustive()
    }
}

impl Makers {
    /// The fill of every made asset a clip shows, asking for the ones not
    /// ready yet — they arrive on a later frame, and until then the clip is
    /// its solid hue.
    pub(super) fn resolve(&mut self, ctx: &Context, open: &Open) -> Fills {
        self.thumbs.poll(ctx);
        self.sounds.poll();
        let mut found = Fills::new();
        for clip in open.project.tracks.iter().flat_map(|track| &track.clips) {
            if found.contains_key(&clip.asset) {
                continue;
            }
            let Some(asset) = open.project.asset(&clip.asset) else {
                continue;
            };
            if !asset.has_renderable_media() {
                continue;
            }
            if let Some(fill) = self.of(ctx, open, asset) {
                found.insert(clip.asset.clone(), fill);
            }
        }
        found
    }

    fn of(&mut self, ctx: &Context, open: &Open, asset: &Asset) -> Option<Fill> {
        let source = open.root.join(asset.path.as_ref()?.as_str());
        if is_sound(asset.kind) {
            let out = thumbs::kept(&open.root, asset, "peaks", "bin");
            return self.sounds.get(ctx, &source, &out).map(Fill::Sound);
        }
        let how = thumbs::how(asset.kind)?;
        let out = thumbs::cached(&open.root, asset);
        self.thumbs
            .get(ctx, &source, how, &out)
            .cloned()
            .map(Fill::Picture)
    }

    /// Forgets everything, for when a different project is opened.
    pub(super) fn reset(&mut self) {
        // Dropping the workers' senders ends their threads once idle.
        *self = Self::default();
    }
}

/// The kinds whose file is a sound, drawn as its waveform.
fn is_sound(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::Audio | AssetKind::GeneratedAudio | AssetKind::SynthAudio
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_sound_is_drawn_as_a_waveform() {
        assert!(is_sound(AssetKind::Audio) && is_sound(AssetKind::SynthAudio));
        assert!(!is_sound(AssetKind::Video) && !is_sound(AssetKind::Text));
    }
}
