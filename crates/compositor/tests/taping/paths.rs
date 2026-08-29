//! The five names a keyframe track can take a tape over by.

use scorsese_compositor::{Properties, path};
use scorsese_core::{
    AssetId, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath, Vhs,
};

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

/// Each of the five reaches its own sub-value and no other. A path wired to the
/// wrong field would leave the picture looking taped and looking wrong, which
/// is exactly the kind of mistake a rendered frame does not announce.
#[test]
fn every_sub_value_has_its_own_path() {
    let tracks = vec![
        constant(path::CHROMA_BLEED, 0.11),
        constant(path::TAPE_NOISE, 0.22),
        constant(path::SCANLINES, 0.33),
        constant(path::JITTER, 0.44),
        constant(path::HEAD_SWITCH, 0.55),
    ];
    let properties = Properties::over(Properties::default(), &tracks, Frames::ZERO);
    assert_eq!(
        properties.vhs,
        Vhs {
            chroma_bleed: 0.11,
            noise: 0.22,
            scanlines: 0.33,
            jitter: 0.44,
            head_switch: 0.55,
            mono: false,
        }
    );
}

/// The field is the baseline and a track takes **one** number over, leaving the
/// other four to the field — the same bargain a grade makes, and what lets
/// "mono throughout, and the tracking goes over the first second" be one line
/// plus one track rather than a choice between them.
#[test]
fn a_track_takes_over_one_number_and_leaves_the_rest_to_the_field() {
    let baseline = Vhs {
        chroma_bleed: 0.4,
        scanlines: 0.3,
        jitter: 0.9,
        mono: true,
        ..Vhs::NONE
    };
    let clip = Clip {
        vhs: baseline,
        keyframes: vec![constant(path::JITTER, 0.05)],
        ..Clip::new(
            ClipId::new("c1"),
            AssetId::new("a"),
            Frames::ZERO,
            Frames(30),
        )
    };
    let properties = Properties::at(&clip, Frames(4));
    assert_eq!(
        properties.vhs,
        Vhs {
            jitter: 0.05,
            ..baseline
        }
    );
    // And the seed is resolved, because there is a tape — the wobble has to
    // move even when nothing else about the clip does.
    assert_ne!(properties.vhs_seed, 0);
}

/// No tape, no seed. A number nothing reads says nothing about the layer, and
/// resolving one anyway would make two instants of an untaped clip compare
/// unequal over a value neither of them uses.
#[test]
fn an_untaped_clip_carries_no_seed() {
    let clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("a"),
        Frames::ZERO,
        Frames(30),
    );
    assert_eq!(Properties::at(&clip, Frames(4)).vhs_seed, 0);
    assert_eq!(
        Properties::at(&clip, Frames(4)),
        Properties::at(&clip, Frames(9))
    );
}
