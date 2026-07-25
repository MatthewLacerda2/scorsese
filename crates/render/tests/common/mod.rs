//! Shared fixtures. Each test file uses a different slice of this, so unused
//! items here are expected rather than dead.
#![allow(dead_code)]

pub mod ffmpeg;

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};
use scorsese_render::plan::Plan;

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

/// A plan as `(timeline start, timeline frames, what fills it, output frames)`
/// per segment — the whole of what sequencing decides, in one comparable value.
///
/// "What fills it" is the clips visible through the stretch, bottom track
/// first, joined by `+`; `gap` when nothing is.
pub fn shape(plan: &Plan<'_>) -> Vec<(u64, u64, String, u64)> {
    plan.segments()
        .iter()
        .map(|segment| {
            let what = if segment.is_gap() {
                "gap".to_owned()
            } else {
                segment
                    .layers
                    .iter()
                    .map(|shot| shot.clip.id.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            };
            (
                segment.start.get(),
                segment.duration.get(),
                what,
                plan.out_frames_of(segment),
            )
        })
        .collect()
}

/// Where each shot opens in its source, by clip id, in segment then track order.
pub fn source_ins(plan: &Plan<'_>) -> Vec<(String, u64)> {
    plan.segments()
        .iter()
        .flat_map(|segment| segment.layers.iter())
        .map(|shot| (shot.clip.id.to_string(), shot.source_in.get()))
        .collect()
}
