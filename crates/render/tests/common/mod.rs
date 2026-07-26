//! Shared fixtures. Each test file uses a different slice of this, so unused
//! items and unused re-exports here are expected rather than dead.
#![allow(dead_code, unused_imports)]

pub mod audio;
pub mod ffmpeg;
pub mod plans;

pub use plans::{audio_shape, shape, source_ins};

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fit, Fps, Frames, MediaMetadata, Project, ProjectPath,
    Track, TrackId, TrackKind,
};

/// An asset with a path under `assets/`, which is what makes the plan treat it
/// as something to decode. The file name follows the id, so a fixture built by
/// [`ffmpeg::generate_asset`] and one built here agree without being told.
pub fn file_asset(id: &str, kind: AssetKind) -> Asset {
    Asset::imported(
        AssetId::new(id),
        kind,
        ProjectPath::new(format!("assets/{id}.{}", extension(kind))),
    )
}

/// The extension the fixtures use for each kind.
pub fn extension(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Image => "png",
        AssetKind::Audio => "wav",
        _ => "mp4",
    }
}

/// A file asset that has been probed and found to have sound on it — a camera
/// clip with dialogue, a screen recording with a click track.
pub fn sounding_asset(id: &str, kind: AssetKind) -> Asset {
    Asset {
        media: Some(MediaMetadata {
            audio_channels: Some(2),
            ..MediaMetadata::default()
        }),
        ..file_asset(id, kind)
    }
}

/// A file asset that has been probed and found to have no audio stream at all.
/// Distinct from one nobody probed: this is an answer, that is the absence of
/// one.
pub fn silent_asset(id: &str, kind: AssetKind) -> Asset {
    Asset {
        media: Some(MediaMetadata::default()),
        ..file_asset(id, kind)
    }
}

/// A prompt-backed asset nobody has spent money on yet.
pub fn sketch_asset(id: &str) -> Asset {
    Asset::sketch(
        AssetId::new(id),
        AssetKind::GeneratedVideo,
        "a skyline at dusk",
    )
}

/// A text asset, which has no file and needs the compositor to draw it.
pub fn text_asset(id: &str) -> Asset {
    Asset {
        text: Some("THE END".to_owned()),
        ..Asset::imported(
            AssetId::new(id),
            AssetKind::Text,
            ProjectPath::new("assets/unused.txt"),
        )
    }
}

pub fn clip(id: &str, asset: &str, start: u64, duration: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        AssetId::new(asset),
        Frames(start),
        Frames(duration),
    )
}

/// A clip that meets the raster some way other than scaling to fit it.
pub fn fitted(fit: Fit, mut clip: Clip) -> Clip {
    clip.fit = fit;
    clip
}

/// A clip that opens partway into its source.
pub fn clip_from(id: &str, asset: &str, start: u64, duration: u64, source_in: u64) -> Clip {
    Clip {
        source_in: Frames(source_in),
        ..clip(id, asset, start, duration)
    }
}

pub fn video_track(id: &str, clips: Vec<Clip>) -> Track {
    track(id, TrackKind::Video, clips)
}

pub fn audio_track(id: &str, clips: Vec<Clip>) -> Track {
    track(id, TrackKind::Audio, clips)
}

pub fn track(id: &str, kind: TrackKind, clips: Vec<Clip>) -> Track {
    Track {
        clips,
        ..Track::new(TrackId::new(id), kind)
    }
}

/// A project on a 30 fps grid. Built directly rather than loaded, so a plan
/// test can hold a timeline that no `project.json` needs to exist for.
pub fn project(assets: Vec<Asset>, tracks: Vec<Track>) -> Project {
    Project {
        assets,
        tracks,
        ..Project::new("fixture", Fps::THIRTY)
    }
}
