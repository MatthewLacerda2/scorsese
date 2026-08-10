//! Brightness and contrast: an offset and a pivot, and which one is which.

use scorsese_core::Grade;

use crate::graded;

/// A grade with the brightness turned and nothing else.
fn brightness(by: f64) -> Grade {
    Grade {
        brightness: by,
        ..Grade::NEUTRAL
    }
}

/// A grade with the contrast turned and nothing else.
fn contrast(to: f64) -> Grade {
    Grade {
        contrast: to,
        ..Grade::NEUTRAL
    }
}

/// `0.2` of the `0.0..=1.0` range is 51 of the 0..255 one, and every channel
/// gets the same 51 — not a share of it proportional to what was there.
#[test]
fn brightness_adds_one_offset_to_every_channel() {
    assert_eq!(graded((10, 100, 200), brightness(0.2)), (61, 151, 251, 255));
}

/// The same offset the other way, and the darkest channel runs out of room
/// before it has given all 51 back.
#[test]
fn negative_brightness_takes_light_away_and_stops_at_black() {
    assert_eq!(graded((10, 100, 200), brightness(-0.2)), (0, 49, 149, 255));
}

/// The difference between an offset and a gain, in one pixel: a gain of any
/// size leaves black black, and this lifts it to mid-grey.
#[test]
fn brightness_lifts_black_rather_than_multiplying_it() {
    assert_eq!(graded((0, 0, 0), brightness(0.5)), (128, 128, 128, 255));
}

/// Mid-grey is the pivot, so no contrast at all is every pixel sitting on the
/// pivot — whatever colour it arrived as.
#[test]
fn contrast_of_zero_collapses_every_colour_onto_mid_grey() {
    assert_eq!(graded((10, 100, 200), contrast(0.0)), (128, 128, 128, 255));
    assert_eq!(graded((255, 255, 255), contrast(0.0)), (128, 128, 128, 255));
}

/// Halved, each channel ends half as far from `0.5` as it started: `0.0` at
/// `0.25`, `200/255` a shade over `0.64`, `1.0` at `0.75`.
#[test]
fn contrast_below_one_flattens_toward_the_pivot() {
    assert_eq!(graded((0, 200, 255), contrast(0.5)), (64, 164, 191, 255));
}

/// And half again as far the other way, until there is no range left to take.
#[test]
fn contrast_above_one_steepens_and_stops_at_both_ends() {
    assert_eq!(graded((0, 100, 200), contrast(1.5)), (0, 86, 236, 255));
    assert_eq!(graded((240, 240, 240), contrast(1.5)), (255, 255, 255, 255));
}

/// A negative contrast would flip the picture about mid-grey, which is not
/// what anybody means by less of it, so it is clamped away before any pixel
/// sees it — landing on the same mid-grey a contrast of zero does.
#[test]
fn a_negative_contrast_is_the_same_as_none_at_all() {
    assert_eq!(graded((10, 100, 200), contrast(-2.0)), (128, 128, 128, 255));
}
