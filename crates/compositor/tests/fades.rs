//! The fade sugar, and the ordinary keyframes it writes.

mod common;

use scorsese_compositor::{fade_in, fade_out, path};
use scorsese_core::Frames;

use common::{clip, opacity_at};

#[test]
fn fade_in_is_ordinary_opacity_keyframes() {
    // The point of the sugar being sugar: what it writes is inspectable and
    // editable, and no renderer needs to know a fade happened.
    let mut clip = clip(30);
    fade_in(&mut clip, Frames(10));

    assert_eq!(clip.keyframes.len(), 1, "one track, on opacity");
    assert_eq!(clip.keyframes[0].property.as_str(), path::OPACITY);
    assert_eq!(opacity_at(&clip, 0), 0.0, "starts from nothing");
    assert_eq!(opacity_at(&clip, 5), 0.5, "halfway up");
    assert_eq!(opacity_at(&clip, 10), 1.0, "solid once the fade is done");
    assert_eq!(opacity_at(&clip, 29), 1.0, "and stays there");
}

#[test]
fn fade_out_reaches_nothing_exactly_on_the_cut() {
    let mut clip = clip(30);
    fade_out(&mut clip, Frames(10));

    assert_eq!(opacity_at(&clip, 0), 1.0);
    assert_eq!(opacity_at(&clip, 20), 1.0, "the fade has not begun");
    assert_eq!(opacity_at(&clip, 25), 0.5);
    // The last frame of the clip is frame 29, and it is still just visible: the
    // picture goes out on the cut rather than a frame before it.
    assert!(opacity_at(&clip, 29) > 0.0);
    assert_eq!(opacity_at(&clip, 30), 0.0);
}

#[test]
fn fading_in_and_out_share_one_sorted_track() {
    let mut clip = clip(30);
    fade_in(&mut clip, Frames(10));
    fade_out(&mut clip, Frames(10));

    assert_eq!(clip.keyframes.len(), 1, "not two tracks on one property");
    let track = &clip.keyframes[0];
    assert!(track.is_sorted(), "validation requires ascending times");
    assert_eq!(track.keyframes.len(), 4);
    assert_eq!(opacity_at(&clip, 0), 0.0);
    assert_eq!(opacity_at(&clip, 15), 1.0, "solid in the middle");
    assert_eq!(opacity_at(&clip, 30), 0.0);
}

#[test]
fn a_fade_of_no_length_does_nothing() {
    let mut clip = clip(30);
    fade_in(&mut clip, Frames::ZERO);
    assert!(clip.keyframes.is_empty(), "no track, no keyframes, no fade");
}

#[test]
fn a_fade_longer_than_its_clip_is_clamped_to_it() {
    let mut clip = clip(10);
    fade_in(&mut clip, Frames(100));
    assert_eq!(opacity_at(&clip, 10), 1.0, "solid by the end of the clip");
    assert_eq!(opacity_at(&clip, 5), 0.5);
}
