//! The three things it must not do — leak into alpha, halo a soft edge, or be
//! skipped — and the two ways a clip asks for it.

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Properties, Resolution, path};
use scorsese_core::{AssetId, Clip, ClipId, Frames};

use super::{SIZE, STRONG, aberrated, pixel, ramp};

/// A track holding one keyframe, so a property has a fixed value.
fn constant(property: &str, value: f64) -> scorsese_core::KeyframeTrack {
    scorsese_core::KeyframeTrack::new(
        scorsese_core::PropertyPath::new(property),
        vec![scorsese_core::Keyframe {
            t: Frames::ZERO,
            value,
            easing: scorsese_core::Easing::Linear,
        }],
    )
}

/// Transparent everywhere but a centred square, with black stored under the
/// transparent part — which is what sampling in straight alpha would drag into
/// the visible edge.
fn badge() -> Frame {
    let edge = SIZE / 4;
    let mut frame = Frame::black(Resolution::source(SIZE, SIZE).expect("a legal source raster"));
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let (x, y) = (index as u32 % SIZE, index as u32 / SIZE);
        let inside = (edge..SIZE - edge).contains(&x) && (edge..SIZE - edge).contains(&y);
        let written = if inside {
            [255, 255, 255, u8::MAX]
        } else {
            [0, 0, 0, 0]
        };
        pixel.copy_from_slice(&written);
    }
    frame
}

/// A layer's shape is its alpha, and pulling its colours apart must not move
/// it. The badge is composited over black, so a channel that had leaked past
/// the alpha would show as a coloured rim outside the square.
#[test]
fn a_soft_edged_layer_keeps_its_own_shape() {
    let frame = aberrated(&badge(), STRONG);
    let edge = SIZE / 4;
    for at in [(edge - 2, SIZE / 2), (SIZE / 2, edge - 2), (2, 2)] {
        assert_eq!(
            pixel(&frame, at.0, at.1),
            (0, 0, 0, u8::MAX),
            "outside the badge at {at:?}: the canvas, untouched"
        );
    }
    assert_eq!(
        pixel(&frame, SIZE / 2, SIZE / 2),
        (255, 255, 255, u8::MAX),
        "and the middle of the badge is still white"
    );
}

/// An aberration too small to move any pixel by half of one leaves the frame
/// byte for byte as it was — not merely close to it.
#[test]
fn a_split_narrower_than_half_a_pixel_is_no_split_at_all() {
    let source = ramp();
    let untouched = aberrated(&source, 0.0);
    for tiny in [0.0001, -1.0, f64::NAN] {
        assert_eq!(
            aberrated(&source, tiny).bytes(),
            untouched.bytes(),
            "an aberration of {tiny} moves nothing"
        );
    }
    assert_ne!(
        aberrated(&source, STRONG).bytes(),
        untouched.bytes(),
        "and one worth having does"
    );
}

/// The copy path exists for a layer that would draw exactly its own pixels, and
/// an aberrated one would not. Left out, the commonest case — a full-frame
/// plate with nothing else on it — would render with no fringing at all.
#[test]
fn an_aberrated_layer_is_never_a_plain_copy() {
    let aberrated = Properties {
        aberration: STRONG,
        ..Properties::default()
    };
    assert!(!aberrated.is_identity());
    assert!(Properties::default().is_identity());
}

/// The clip's **own** fields reach the properties it resolves to, which is the
/// path a render actually takes.
///
/// Everything else here goes through [`Properties::over`], which never sees a
/// [`Clip`] — so without this, a clip could carry a baseline aberration and a
/// baseline blur and composite as though it carried neither, with the whole
/// suite green. The two are asserted together because they are the same line of
/// the same struct literal.
#[test]
fn a_clips_own_numbers_reach_the_properties_it_resolves_to() {
    let mut clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(30),
    );
    clip.blur = 0.02;
    clip.aberration = STRONG;

    let properties = Properties::at(&clip, Frames(7));
    assert!((properties.blur - 0.02).abs() < f64::EPSILON, "blur");
    assert!(
        (properties.aberration - STRONG).abs() < f64::EPSILON,
        "aberration"
    );
}

/// A field *and* an animatable property: the field is the clip's baseline, a
/// track takes it over for the whole clip.
#[test]
fn aberration_is_a_baseline_and_a_track_takes_it_over() {
    let baseline = Properties {
        aberration: 0.03,
        ..Properties::default()
    };
    assert!((Properties::over(baseline, &[], Frames::ZERO).aberration - 0.03).abs() < f64::EPSILON);

    let animated = Properties::over(baseline, &[constant(path::ABERRATION, 0.5)], Frames::ZERO);
    assert!((animated.aberration - 0.5).abs() < f64::EPSILON);
}
