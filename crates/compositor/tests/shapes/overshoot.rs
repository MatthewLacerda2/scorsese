//! A trim keyframed with an easing that overshoots (#587) — the reason a trim
//! is clamped rather than refused.
//!
//! `back_out` from `0` to `1` carries `shape.trim_end` past `1` on its way in,
//! and `back_in` dips it below `0` on its way out. Neither is a mistake in the
//! document; each has to draw exactly what its clamped value draws.

use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, Stroking, draw_stroked};
use scorsese_compositor::{Frame, Properties, path};
use scorsese_core::{Curve, Easing, Frames, Heads, Keyframe, KeyframeTrack, PropertyPath};

use crate::extent::extent;
use crate::{BLUE, frame};

fn arrow() -> Figure {
    Figure {
        outline: Outline::Arrow(Arrow {
            from: (40.0, 100.0),
            to: (160.0, 100.0),
            curve: Curve::Straight,
            heads: Heads::End,
        }),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: 4.0,
        }),
    }
}

/// `shape.trim_end` from 0 at frame 0 to 1 at frame 20, eased by `easing`.
fn drawing_on(easing: Easing) -> KeyframeTrack {
    let key = |t: u64, value: f64, easing: Easing| Keyframe {
        t: Frames(t),
        value,
        easing,
    };
    KeyframeTrack::new(
        PropertyPath::new(path::TRIM_END),
        vec![key(0, 0.0, easing), key(20, 1.0, Easing::Linear)],
    )
}

/// The trim the track resolves to at `t`, and the frame it draws.
fn at(track: &KeyframeTrack, t: u64) -> (f64, Frame) {
    let trace = Properties::over(
        Properties::default(),
        std::slice::from_ref(track),
        Frames(t),
    )
    .trace;
    let stroking = Stroking {
        trim_start: trace.trim_start as f32,
        trim_end: trace.trim_end as f32,
        dash: None,
    };
    let mut frame = frame();
    draw_stroked(&mut frame, &arrow(), &stroking);
    (trace.trim_end, frame)
}

#[test]
fn a_trim_eased_past_the_end_draws_the_whole_line() {
    let track = drawing_on(Easing::BackOut);
    let (_, whole) = at(&track, 20);
    let past: Vec<u64> = (1..20).filter(|&t| at(&track, t).0 > 1.0).collect();
    assert!(
        !past.is_empty(),
        "back_out overshoots somewhere before it lands"
    );
    for t in past {
        assert_eq!(at(&track, t).1.bytes(), whole.bytes(), "frame {t}");
    }
}

#[test]
fn a_trim_eased_below_the_start_draws_nothing() {
    let track = drawing_on(Easing::BackIn);
    let under: Vec<u64> = (1..20).filter(|&t| at(&track, t).0 < 0.0).collect();
    assert!(!under.is_empty(), "back_in dips before it sets off");
    for t in under {
        assert_eq!(extent(&at(&track, t).1), None, "frame {t}");
    }
}
