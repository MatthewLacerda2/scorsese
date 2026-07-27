//! Sequencing a timeline into a render, with no ffmpeg anywhere in sight.
//!
//! Everything about *what is on screen when* is decided here, so this is where
//! it gets pinned down exhaustively — no encoding, no temp files, no external
//! binary.

#[path = "../common/mod.rs"]
mod common;

mod audio;
mod embedded;
mod layering;
mod range;
mod refusals;
mod resuming;
mod sequencing;
mod sketches;

use scorsese_core::{Asset, AssetKind, Project};

use common::{clip, file_asset, project, video_track};

/// Two plain video assets, for a lower track and an upper one.
fn two_videos() -> Vec<Asset> {
    vec![
        file_asset("under", AssetKind::Video),
        file_asset("over", AssetKind::Video),
    ]
}

/// A full-length lower track with a shorter clip sitting over its middle, which
/// is the shape most multi-track questions reduce to.
fn overlaid() -> Project {
    project(
        two_videos(),
        vec![
            video_track("v1", vec![clip("bed", "under", 0, 60)]),
            video_track("v2", vec![clip("top", "over", 20, 20)]),
        ],
    )
}
