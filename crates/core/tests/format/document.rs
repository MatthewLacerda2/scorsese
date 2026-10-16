//! One project with every field the format has set, so serialising it writes
//! every key `project.json` can hold.
//!
//! **Every struct is a literal with no `..`**: a field added to the format
//! does not compile here until this document sets it, which is what keeps the
//! fingerprint from missing exactly the change it exists for. Each value is
//! set away from its default, since a default is skipped when written. Enum
//! fields take one variant each; [`super::variants`] lists the rest.

use scorsese_core::{
    Aspect, Asset, AssetId, AssetKind, Baseline, ClipSeconds, Counter, Fill, FontChoice, Fps,
    Frames, GenerationState, Group, Icon, ImageAspect, ImageRequest, ImageResolution,
    ImageSequence, ImageThinking, Locale, MediaMetadata, Project, ProjectPath, Ratio, Reveal,
    RevealUnit, Rgba, SCHEMA_VERSION, Shape, SpeechModel, SpeechRequest, TextAlign, TextStyle,
    Timestamp, Track, TrackId, TrackKind, VideoModel, VideoRequest, VideoResolution,
};

use super::clip::clip;
use super::variants::{arrow, linear};

/// The document: one asset, one track holding one clip.
pub(crate) fn project() -> Project {
    Project {
        schema_version: SCHEMA_VERSION,
        name: "Every field".to_owned(),
        timeline_fps: Fps::new(30, 1).expect("30 fps"),
        aspect: Ratio::new(4, 5).expect("a portrait frame"),
        script: Some(ProjectPath::new("script.md")),
        assets: vec![asset()],
        tracks: vec![track()],
        baseline: Baseline::unread(),
    }
}

pub(crate) fn track() -> Track {
    Track {
        id: TrackId::new("v1"),
        kind: TrackKind::Video,
        name: Some("Picture".to_owned()),
        note: Some("Why this track".to_owned()),
        clips: vec![clip()],
    }
}

pub(crate) fn color() -> Rgba {
    Rgba::new(0x0b, 0x10, 0x20, 0xff)
}

fn stamp() -> Timestamp {
    Timestamp::try_from("2026-10-16T12:00:00Z".to_owned()).expect("an instant")
}

fn asset() -> Asset {
    Asset {
        id: AssetId::new("everything"),
        kind: AssetKind::GeneratedVideo,
        path: Some(ProjectPath::new("generated/everything.mp4")),
        sha256: Some("ab".repeat(32)),
        media: Some(media()),
        prompt: Some("A prompt".to_owned()),
        recipe: Some(ProjectPath::new("recipes/everything.json")),
        state: Some(GenerationState::Generated),
        text: Some("Some text".to_owned()),
        style: Some(style()),
        color: Some(Fill::Linear(linear())),
        shape: Some(Shape {
            geometry: arrow(),
            fill: Some(Fill::Solid(color())),
            stroke: Some(color()),
            stroke_width: 3.5,
            dash: Some(vec![4.0, 2.0]),
        }),
        icon: Some(Icon {
            name: "star".to_owned(),
            size: 64.0,
            color: color(),
            stroke_width: 1.5,
        }),
        group: Some(Group {
            tracks: vec![Track {
                id: TrackId::new("inner"),
                kind: TrackKind::Audio,
                name: None,
                note: None,
                clips: Vec::new(),
            }],
        }),
        sequence: Some(ImageSequence {
            stills: vec![AssetId::new("still")],
            hold: Frames(2),
            looping: true,
        }),
        note: Some("Why this asset".to_owned()),
        video: Some(video()),
        speech: Some(SpeechRequest {
            model: SpeechModel::Expressive,
            voice_id: Some("voice".to_owned()),
            language: Some("pt".to_owned()),
            seed: Some(7),
        }),
        image: Some(ImageRequest {
            resolution: Some(ImageResolution::K2),
            aspect: ImageAspect::Tall,
            thinking: Some(ImageThinking::High),
            reference_images: vec![AssetId::new("reference")],
            character_images: vec![AssetId::new("character")],
        }),
        created_at: Some(stamp()),
        queued_at: Some(stamp()),
        operation: Some("operations/1".to_owned()),
        estimated_cost_cents: Some(120),
        source: Some(super::stock::source()),
    }
}

fn media() -> MediaMetadata {
    MediaMetadata {
        duration_seconds: Some(8.0),
        width: Some(1920),
        height: Some(1080),
        frame_rate: Some(Fps::new(24, 1).expect("24 fps")),
        has_alpha: Some(true),
        audio_channels: Some(2),
        sample_rate: Some(48_000),
    }
}

fn style() -> TextStyle {
    TextStyle {
        font: FontChoice::Named("serif".to_owned()),
        weight: Some(700),
        italic: true,
        size: 48.0,
        color: color(),
        align: TextAlign::Left,
        line_height: 1.25,
        max_width: 0.75,
        stroke: Some(color()),
        stroke_width: 2.0,
        reveal: Some(Reveal {
            unit: RevealUnit::Char,
            rise: 0.5,
            stagger: 0.25,
        }),
        number: Some(Counter {
            value: 1234.5,
            decimals: 1,
            locale: Locale::PtBr,
            grouping: true,
        }),
    }
}

fn video() -> VideoRequest {
    VideoRequest {
        model: VideoModel::Standard,
        resolution: VideoResolution::P4k,
        seconds: ClipSeconds::Four,
        aspect: Aspect::Tall,
        first_image: Some(AssetId::new("first")),
        last_image: Some(AssetId::new("last")),
        reference_images: vec![AssetId::new("reference")],
        seed: Some(11),
    }
}
