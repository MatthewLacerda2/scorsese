//! Where in its own animation a held image is at a given instant.
//!
//! An `image` asset is held for its clip's length, and a held gif or avif plays
//! its frames and starts again whenever it runs out. A decode that begins at
//! the clip's start gets that for free — the animation starts with the clip.
//! A decode that begins anywhere else does not: a preview of one frame, a
//! partial render, or a segment opened mid-clip by a cut on another track. Each
//! has to be told how far into the animation the instant it starts at falls,
//! and that is `elapsed % length`, which needs the animation's own length.
//!
//! **That length is measured here, at render time, and never written down.**
//! The assets table deliberately drops an image's duration
//! ([`scorsese_core::MediaMetadata::as_recorded`]): ffprobe invents one for a
//! still, and how long an image is on screen is the clip's business, not the
//! file's. So the one render that needs the number asks the file, the way
//! [`crate::raster::Sizes`] asks for a picture size the document does not
//! hold — once per asset, before any frame is decoded, and only for a shot that
//! actually starts mid-clip.

use std::collections::HashMap;
use std::path::Path;

use scorsese_core::{AssetId, AssetKind, Fps, ProbeMedia};

use crate::pipe::reads_through_image2;
use crate::plan::{Plan, Shot};
use crate::probe::Ffprobe;
use crate::slug::Standing;
use crate::tools::Tools;

/// The length, in seconds, of every held animation a render opens part-way
/// through.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Loops {
    lengths: HashMap<AssetId, f64>,
}

impl Loops {
    /// Measures every held animation in this plan that some stretch of it opens
    /// part-way through.
    ///
    /// A file that cannot be measured is left out rather than refused. Nothing
    /// is wrong with the edit: without the length a decode is told the plain
    /// elapsed time instead, which lands on the same frame and only costs
    /// decoding the loops before it. If the file is truly unreadable, the
    /// decoder says so in its own words a moment later.
    pub(crate) fn measure(tools: &Tools, plan: &Plan<'_>, project_root: &Path) -> Self {
        let probe = Ffprobe::new(tools.clone());
        let mut lengths = HashMap::new();
        for shot in plan.segments().iter().flat_map(|segment| segment.shots()) {
            if lengths.contains_key(&shot.asset.id) || elapsed_frames(shot) <= 0.0 {
                continue;
            }
            let Some(file) = looped_file(shot, project_root) else {
                continue;
            };
            let length = probe
                .probe(&file)
                .ok()
                .and_then(|media| media.duration_seconds)
                .filter(|length| length.is_finite() && *length > 0.0);
            if let Some(length) = length {
                lengths.insert(shot.asset.id.clone(), length);
            }
        }
        Self { lengths }
    }

    /// How far into its own animation, in seconds, a held shot's decode
    /// starts. Zero for a shot that starts with its clip — the case every
    /// render of a whole clip is, and the one whose ffmpeg command must not
    /// change.
    ///
    /// The clip's speed is undone rather than applied: a held picture plays at
    /// its own rate whatever the clip's speed says ("held is held", in the
    /// decoder's filter), so what has elapsed of the animation is what has
    /// elapsed of the timeline.
    ///
    /// Exact when the animation is a whole number of output frames long. When
    /// it is not, the full render's frame grid drifts against the loop and the
    /// modulo restarts it, so a mid-clip start can land one output frame from
    /// where the whole render has it — a gap smaller than the gif's own frames.
    pub(crate) fn seconds_in(&self, shot: &Shot<'_>, timeline_fps: Fps) -> f64 {
        let elapsed = timeline_fps.seconds_at(elapsed_frames(shot));
        if elapsed <= 0.0 {
            return 0.0;
        }
        self.lengths
            .get(&shot.asset.id)
            .map_or(elapsed, |length| elapsed % length)
    }
}

/// How many timeline frames into its clip a shot begins.
///
/// Recovered from the shot's own `source_in`, which the plan worked out at the
/// clip's speed, rather than carried separately — one source of truth for
/// where a stretch of a clip begins.
fn elapsed_frames(shot: &Shot<'_>) -> f64 {
    let into_source = shot.source_in - shot.clip.source_in.get() as f64;
    shot.clip.speed.timeline_frames(into_source)
}

/// The file of a shot that is held by looping its container — a gif, an avif,
/// anything not read through ffmpeg's `image2` demuxer — when it is on disk.
///
/// An `image2` still is one picture repeated, which looks the same from
/// anywhere, so there is nothing to measure for it.
fn looped_file(shot: &Shot<'_>, project_root: &Path) -> Option<std::path::PathBuf> {
    if shot.asset.kind != AssetKind::Image {
        return None;
    }
    match crate::slug::standing(shot, project_root) {
        Ok(Standing::Media(file)) if !reads_through_image2(&file) => Some(file),
        _ => None,
    }
}
