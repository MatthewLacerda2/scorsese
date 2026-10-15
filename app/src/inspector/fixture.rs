//! A project for the inspector's own tests to edit.
//!
//! Shared rather than rebuilt in each test module, because the interesting part
//! of a test here is the *change* — and a fixture written twice is a fixture
//! that drifts until two tests are quietly about different documents.

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, Project, ProjectPath, Track, TrackKind};

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

    project.tracks = vec![
        Track::with_clips(
            "v1",
            TrackKind::Video,
            vec![
                Clip::at("c1", "shot", 0, 90),
                // Starts well after the first ends, so there is room to move into.
                Clip::at("c2", "shot", 120, 60),
            ],
        ),
        Track::with_clips(
            "a1",
            TrackKind::Audio,
            vec![Clip::at("c3", "music", 0, 180)],
        ),
    ];
    project
}
