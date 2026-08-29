//! How a clip's key reaches the properties one instant of it resolves to.

use scorsese_compositor::{Properties, path};
use scorsese_core::{AssetId, ChromaKey, Clip, ClipId, Frames, KeyframeTrack, PropertyPath};
use scorsese_core::{Easing, Keyframe};

use super::SCREEN;

fn clip() -> Clip {
    Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(30),
    )
}

/// A track holding one value for the whole clip.
fn constant(property: &str, value: f64) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value,
            easing: Easing::Linear,
        }],
    )
}

/// The clip's own key reaches the properties it resolves to, which is the path
/// a render actually takes.
///
/// Everything else in this directory goes through [`Properties::over`], which
/// never sees a `Clip` — so without this a clip could carry a key and composite
/// as though it carried none, with the whole suite green.
#[test]
fn a_clips_own_key_reaches_the_properties_it_resolves_to() {
    let bare = clip();
    let mut keyed = clip();
    keyed.chroma_key = Some(ChromaKey::new(SCREEN));
    assert_eq!(
        Properties::at(&keyed, Frames(7)).chroma_key,
        Some(ChromaKey::new(SCREEN))
    );
    assert_eq!(Properties::at(&bare, Frames(7)).chroma_key, None);
}

/// The two numbers are a baseline *and* animatable, like a grade: the field is
/// what the clip says once, a track takes that one property over for the whole
/// clip, and the other three settings go on being what the field said.
#[test]
fn a_track_takes_one_setting_over_and_leaves_the_rest() {
    let mut clip = clip();
    clip.chroma_key = Some(ChromaKey {
        color: SCREEN,
        tolerance: 0.2,
        softness: 0.05,
        spill: true,
    });
    clip.keyframes = vec![constant(path::KEY_TOLERANCE, 0.44)];

    let key = Properties::at(&clip, Frames(7))
        .chroma_key
        .expect("the key is still there");
    assert!((key.tolerance - 0.44).abs() < f64::EPSILON, "taken over");
    assert!((key.softness - 0.05).abs() < f64::EPSILON, "left alone");
    assert_eq!(key.color, SCREEN, "and so is the screen colour");
    assert!(key.spill, "and the suppression");

    let softened = constant(path::KEY_SOFTNESS, 0.33);
    let key = Properties::over(Properties::at(&clip, Frames::ZERO), &[softened], Frames::ZERO)
        .chroma_key
        .expect("the key is still there");
    assert!((key.softness - 0.33).abs() < f64::EPSILON);
}

/// A tolerance track on a clip with no key does nothing at all, and does not
/// invent a key to hang itself on — which would key against whatever colour the
/// invention happened to be.
#[test]
fn a_setting_without_a_key_is_a_number_about_nothing() {
    let mut clip = clip();
    clip.keyframes = vec![
        constant(path::KEY_TOLERANCE, 0.44),
        constant(path::KEY_SOFTNESS, 0.33),
    ];
    let properties = Properties::at(&clip, Frames(7));
    assert_eq!(properties.chroma_key, None);
    assert_eq!(properties, Properties::default());
}

/// A keyed layer is never a plain copy of its own pixels, however untouched
/// everything else about it is.
#[test]
fn a_key_alone_is_enough_to_stop_a_layer_being_a_copy() {
    let keyed = Properties {
        chroma_key: Some(ChromaKey::new(SCREEN)),
        ..Properties::default()
    };
    assert!(!keyed.is_identity());
    assert!(Properties::default().is_identity());
}
