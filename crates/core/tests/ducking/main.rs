//! Auto-ducking, asserted on the **keyframes produced** rather than on any
//! sound.
//!
//! That is the whole contract: ducking is a keyframe generator, so the thing
//! worth pinning is exactly where the four points of each dip land and what
//! they carry. None of it needs ffmpeg, and none of it needs audio to exist.

#[path = "../common/mod.rs"]
mod common;

mod rerun;
mod shape;

use scorsese_core::{Clip, ClipId, Dip, Fps, Frames, Project, Track, TrackId, TrackKind};

/// A project with a 600-frame music clip on `music`, and narration clips at
/// the given `(start, duration)` frames on `vo`.
pub(crate) fn scored(narration: &[(u64, u64)]) -> Project {
    let mut project = Project::new("ducking", Fps::THIRTY);
    project.tracks = vec![
        audio_track("music", vec![clip("m1", 0, 600)]),
        audio_track(
            "vo",
            narration
                .iter()
                .enumerate()
                .map(|(i, (start, duration))| clip(&format!("v{i}"), *start, *duration))
                .collect(),
        ),
    ];
    project
}

pub(crate) fn audio_track(id: &str, clips: Vec<Clip>) -> Track {
    Track {
        id: TrackId::new(id),
        kind: TrackKind::Audio,
        name: None,
        note: None,
        clips,
    }
}

pub(crate) fn clip(id: &str, start: u64, duration: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        scorsese_core::AssetId::new("a"),
        Frames(start),
        Frames(duration),
    )
}

/// The default dip on a 30 fps grid: 9 frames of attack, 18 of release.
pub(crate) fn dip() -> Dip {
    Dip::default_for(30.0)
}

/// The music clip's keyframe points as `(frame, value)`, for the one track
/// ducking wrote.
pub(crate) fn points(project: &Project) -> Vec<(u64, f64)> {
    project.tracks[0].clips[0]
        .keyframes
        .iter()
        .find(|track| track.is_generated_by("duck"))
        .map(|track| {
            track
                .keyframes
                .iter()
                .map(|k| (k.t.get(), k.value))
                .collect()
        })
        .unwrap_or_default()
}
