//! Which keyframed properties make a shape a different picture at every
//! instant — the answer that decides whether the render draws it once for a
//! segment or again for every frame.
//!
//! Wrong one way, a keyframed trim is drawn once and frozen: a draw-on that
//! never draws on. Wrong the other, every shape is redrawn every frame for
//! nothing. Both have to be pinned, because either passes a test that only
//! checks the other.

use scorsese_compositor::path;
use scorsese_compositor::shape::Trace;
use scorsese_core::{Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

fn track(property: &str) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value: 0.5,
            easing: Easing::Linear,
        }],
    )
}

#[test]
fn a_keyframed_trim_or_dash_offset_redraws_the_shape() {
    for property in [path::TRIM_START, path::TRIM_END, path::DASH_OFFSET] {
        let tracks = [track(path::OPACITY), track(property)];
        assert!(Trace::is_animated_by(&tracks), "{property}");
    }
}

#[test]
fn a_shape_whose_line_is_not_keyframed_is_drawn_once() {
    assert!(!Trace::is_animated_by(&[]));
    let moved = [track(path::OPACITY), track(path::POSITION_X)];
    assert!(!Trace::is_animated_by(&moved));
}
