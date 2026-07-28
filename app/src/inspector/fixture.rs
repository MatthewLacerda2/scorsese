//! A project for the inspector's own tests to edit.
//!
//! Shared rather than rebuilt in each test module, because the interesting part
//! of a test here is the *change* — and a fixture written twice is a fixture
//! that drifts until two tests are quietly about different documents.

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};

/// Two clips on a video track with a gap between them, and one on an audio
/// track. Valid, and it stays valid until a test breaks it on purpose.
///
/// The gap is the point: it is what a start or duration can be dragged into,
/// and what it can be dragged *past* to earn a refusal.
pub(super) fn project() -> Project {
    let mut project = Project::new("test", Fps::THIRTY);
    project.assets = vec![
        Asset::imported(
            AssetId::new("shot"),
            AssetKind::Video,
            ProjectPath::new("assets/shot.mp4"),
        ),
        Asset::imported(
            AssetId::new("music"),
            AssetKind::Audio,
            ProjectPath::new("assets/music.wav"),
        ),
    ];

    let mut video = Track::new(TrackId::new("v1"), TrackKind::Video);
    video.clips = vec![
        clip("c1", "shot", 0, 90),
        // Starts well after the first ends, so there is room to move into.
        clip("c2", "shot", 120, 60),
    ];
    let mut audio = Track::new(TrackId::new("a1"), TrackKind::Audio);
    audio.clips = vec![clip("c3", "music", 0, 180)];
    project.tracks = vec![video, audio];
    project
}

fn clip(id: &str, asset: &str, start: u64, duration: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        AssetId::new(asset),
        Frames(start),
        Frames(duration),
    )
}
