//! The clip of [`super::document`], every field set away from its default.

use scorsese_core::{
    Anchor, AnchorX, AnchorY, AssetId, Blend, ChromaKey, Clip, ClipId, Crop, Easing, Fit, Follow,
    Frames, Glow, Grade, Keyframe, KeyframeTrack, Matte, Origin, OriginX, OriginY, PropertyPath,
    Shadow, Speed, Vhs,
};

use super::document::color;

pub(crate) fn clip() -> Clip {
    Clip {
        id: ClipId::new("c1"),
        asset: AssetId::new("everything"),
        start: Frames(30),
        duration: Frames(90),
        source_in: Frames(15),
        speed: Speed::new(1.5),
        fit: Fit::Fill,
        crop: Some(Crop {
            x: 0.125,
            y: 0.25,
            width: 0.5,
            height: 0.5,
        }),
        anchor: Anchor {
            x: AnchorX::Left,
            y: AnchorY::Bottom,
        },
        origin: Origin {
            x: OriginX::Right,
            y: OriginY::Top,
        },
        grade: grade(),
        blur: 2.0,
        aberration: 0.5,
        chroma_key: Some(ChromaKey {
            color: color(),
            tolerance: 0.25,
            softness: 0.125,
            spill: true,
        }),
        vhs: vhs(),
        shadow: Some(Shadow {
            color: color(),
            offset_x: 4.0,
            offset_y: 6.0,
            softness: 8.0,
            opacity: 0.5,
        }),
        glow: Some(Glow {
            color: Some(color()),
            radius: 12.0,
            intensity: 0.75,
        }),
        blend: Blend::Screen,
        matte: Some(Matte {
            clip: ClipId::new("mask"),
            invert: true,
        }),
        note: Some("Why this clip".to_owned()),
        follow: Some(Follow {
            clip: ClipId::new("leader"),
            orient: true,
        }),
        keyframes: vec![KeyframeTrack {
            property: PropertyPath::new("transform.x"),
            by: Some("Why it moves".to_owned()),
            keyframes: vec![Keyframe {
                t: Frames(0),
                value: 0.25,
                easing: Easing::EaseInOut,
            }],
        }],
    }
}

fn grade() -> Grade {
    Grade {
        saturation: 0.5,
        temperature: 0.25,
        brightness: 0.125,
        contrast: 1.25,
        vignette: 0.5,
        grain: 0.25,
    }
}

fn vhs() -> Vhs {
    Vhs {
        chroma_bleed: 0.5,
        noise: 0.25,
        scanlines: 0.5,
        jitter: 0.125,
        head_switch: 0.25,
        mono: true,
    }
}
