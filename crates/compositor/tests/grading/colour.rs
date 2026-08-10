//! Saturation and temperature: which grey a pixel is measured against, and
//! what warming one does to the channels it is not about.

use scorsese_core::Grade;

use crate::graded;

/// A grade with the saturation turned and nothing else.
fn saturation(to: f64) -> Grade {
    Grade {
        saturation: to,
        ..Grade::NEUTRAL
    }
}

/// A grade with the temperature turned and nothing else.
fn temperature(to: f64) -> Grade {
    Grade {
        temperature: to,
        ..Grade::NEUTRAL
    }
}

/// The Rec.709 weights, one primary at a time — `0.2126`, `0.7152`, `0.0722`
/// of full scale. A plain average of the three channels would send all three
/// of these to 85, and a green field would go grey at a third of the
/// brightness it looks.
#[test]
fn full_desaturation_leaves_each_primary_its_own_rec709_luma() {
    assert_eq!(graded((255, 0, 0), saturation(0.0)), (54, 54, 54, 255));
    assert_eq!(graded((0, 255, 0), saturation(0.0)), (182, 182, 182, 255));
    assert_eq!(graded((0, 0, 255), saturation(0.0)), (18, 18, 18, 255));
}

/// Halfway is halfway: green's own grey is `0.7152`, and each channel lands
/// midway between where it was and there.
#[test]
fn saturation_scales_how_far_a_channel_sits_from_that_grey() {
    assert_eq!(graded((0, 255, 0), saturation(0.5)), (91, 219, 91, 255));
}

/// Above one it pushes the other way, out past where the channel started.
#[test]
fn saturation_above_one_pushes_further_from_that_grey() {
    assert_eq!(graded((100, 150, 100), saturation(2.0)), (64, 164, 64, 255));
}

/// Below zero it would invert the colour about that grey, which nobody means
/// by less of it, so it is clamped to none at all.
#[test]
fn a_negative_saturation_is_the_same_as_none_at_all() {
    assert_eq!(graded((0, 255, 0), saturation(-1.0)), (182, 182, 182, 255));
}

/// A quarter either way at `1.0`: red gains a quarter, blue loses one, and
/// green — which carries most of the luma — is not touched at all.
#[test]
fn temperature_gains_red_and_loses_blue_by_a_quarter() {
    let grey = (128, 128, 128);
    assert_eq!(graded(grey, temperature(1.0)), (160, 128, 96, 255));
    assert_eq!(graded(grey, temperature(-1.0)), (96, 128, 160, 255));
}

/// A gain and not an offset, which is the whole reason it is written that way:
/// warming a shot must not lay a red fog over its shadows. Black stays black,
/// while white has its blue pulled down to `0.75` and its red clipped off.
#[test]
fn warming_leaves_black_exactly_black() {
    assert_eq!(graded((0, 0, 0), temperature(1.0)), (0, 0, 0, 255));
    assert_eq!(
        graded((255, 255, 255), temperature(1.0)),
        (255, 255, 191, 255)
    );
}
