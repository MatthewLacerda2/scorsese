//! The three animatable numbers, and what a clip carries into [`Properties`].

use scorsese_compositor::{ANIMATED, MAX_GLOW_INTENSITY, Properties, path};
use scorsese_core::{
    AssetId, Blend, Clip, ClipId, Easing, Frames, Glow, Keyframe, KeyframeTrack, PropertyPath,
    Shadow,
};

fn held(property: &str, value: f64) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value,
            easing: Easing::Linear,
        }],
    )
}

fn clip() -> Clip {
    Clip::new(
        ClipId::new("c"),
        AssetId::new("a"),
        Frames::ZERO,
        Frames(10),
    )
}

#[test]
fn a_clip_carries_its_light_and_its_blend() {
    let mut clip = clip();
    clip.shadow = Some(Shadow::default());
    clip.glow = Some(Glow::default());
    clip.blend = Blend::Screen;
    let properties = Properties::at(&clip, Frames::ZERO);
    assert_eq!(properties.shadow, Some(Shadow::default()));
    assert_eq!(properties.glow, Some(Glow::default()));
    assert_eq!(properties.blend, Blend::Screen);
    assert!(
        !properties.is_identity(),
        "a lit layer is never a plain copy"
    );
}

#[test]
fn a_track_takes_its_number_over_only_when_there_is_one_to_take() {
    let mut clip = clip();
    clip.keyframes = vec![
        held(path::SHADOW_OPACITY, 0.9),
        held(path::GLOW_RADIUS, 0.07),
        held(path::GLOW_INTENSITY, 2.5),
    ];
    let bare = Properties::at(&clip, Frames::ZERO);
    assert_eq!((bare.shadow, bare.glow), (None, None), "none invented");
    assert!(bare.is_identity());

    clip.shadow = Some(Shadow::default());
    clip.glow = Some(Glow::default());
    let lit = Properties::at(&clip, Frames::ZERO);
    let (shadow, glow) = (lit.shadow.expect("a shadow"), lit.glow.expect("a glow"));
    assert!((shadow.opacity - 0.9).abs() < f64::EPSILON);
    assert!((glow.radius - 0.07).abs() < f64::EPSILON);
    assert!((glow.intensity - 2.5).abs() < f64::EPSILON);
}

/// Each blend but `normal` reads the canvas, so none of them is a copy.
#[test]
fn only_a_normal_blend_can_be_copied() {
    for blend in [Blend::Add, Blend::Screen, Blend::Multiply] {
        let properties = Properties {
            blend,
            ..Properties::default()
        };
        assert!(!properties.is_identity(), "{}", blend.as_str());
    }
}

/// The ceiling the glow's description quotes is the one the compositor clamps
/// to — the two are written in different places, so this keeps them agreeing.
#[test]
fn the_glow_intensity_description_quotes_the_real_ceiling() {
    let described = ANIMATED
        .iter()
        .find(|property| property.path == path::GLOW_INTENSITY)
        .expect("glow.intensity is animated");
    let ceiling = format!("0-{MAX_GLOW_INTENSITY}");
    assert!(
        described.describes.contains(&ceiling),
        "{}",
        described.describes
    );
}
