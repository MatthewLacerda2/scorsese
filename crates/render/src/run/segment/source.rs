//! What a decoder is told about the shot it reads: which file, from where, and
//! how it meets the raster.
//!
//! Kept apart from [`super::layers`] because where a decode starts is the one
//! question with two different answers — a played source seeks into its media,
//! a held one into its own animation — and both are settled here.

use scorsese_core::{AssetKind, Fps};

use crate::held::Loops;
use crate::pipe::{Fitting, Source};
use crate::plan::Shot;

/// How to read a shot's media, given where it is.
pub(super) fn source_for(
    shot: &Shot<'_>,
    file: std::path::PathBuf,
    loops: &Loops,
    timeline_fps: Fps,
    frames: u64,
    fitting: Fitting,
) -> Source {
    // A still has no timeline of its own: it is held for the clip's length
    // rather than played, so what it seeks into is its own animation, if it
    // has one — never the clip's place in the source.
    let still = shot.asset.kind == AssetKind::Image;
    Source {
        file,
        still,
        seek_seconds: if still {
            loops.seconds_in(shot, timeline_fps)
        } else {
            timeline_fps.seconds_at(shot.source_in)
        },
        speed: shot.clip.speed,
        frames,
        fitting,
        // Read off the document, which is where a probe writes it. An asset
        // nobody probed reads as opaque — the same answer, and the same filter
        // chain, as before there was a field to read.
        has_alpha: shot
            .asset
            .media
            .and_then(|media| media.has_alpha)
            .unwrap_or(false),
        crop: shot.clip.crop,
    }
}
