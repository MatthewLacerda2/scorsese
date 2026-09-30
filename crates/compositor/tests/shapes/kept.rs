//! An end the trim did not move keeps the head it always had.
//!
//! A trimmed line is measured as a flattened polyline, so a head placed from
//! the measurement is aimed down the first or last straight piece — a chord of
//! the curve, a few degrees off its true tangent on a bowed arrow. An end the
//! trim never touched is placed from the arrow's own geometry instead, so
//! trimming one end of a bowed arrow does not nudge the head at the other.
//!
//! On a straight arrow the chord *is* the tangent, which is why this needs a
//! short S: few pieces, and a first piece that visibly cuts the corner.

use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, Stroking, draw, draw_stroked};
use scorsese_core::{Curve, Heads};

use crate::extent::{Extent, extent};
use crate::{BLUE, frame};

fn short_s(heads: Heads) -> Figure {
    Figure {
        outline: Outline::Arrow(Arrow {
            from: (90.0, 80.0),
            to: (110.0, 120.0),
            curve: Curve::S,
            heads,
        }),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: 8.0,
        }),
    }
}

fn drawn(heads: Heads, trim: Option<(f32, f32)>) -> Extent {
    let mut drawn = frame();
    match trim {
        None => draw(&mut drawn, &short_s(heads)),
        Some((trim_start, trim_end)) => {
            let stroking = Stroking {
                trim_start,
                trim_end,
                dash: None,
            };
            draw_stroked(&mut drawn, &short_s(heads), &stroking);
        }
    }
    extent(&drawn).expect("a line with ink")
}

/// The start head is the leftmost ink and the end head the rightmost: trimming
/// the other end must not move either by a pixel. (Not the lowest — the rebuilt
/// line's last piece rounds one pixel differently there, and that is the line,
/// not the head.)
#[test]
fn trimming_one_end_of_a_bowed_arrow_leaves_the_other_head_where_it_was() {
    let whole = drawn(Heads::Both, None);
    let end_trimmed = drawn(Heads::Both, Some((0.0, 0.8)));
    assert_eq!(end_trimmed.left, whole.left, "the start head held still");

    let whole = drawn(Heads::End, None);
    let start_trimmed = drawn(Heads::End, Some((0.2, 1.0)));
    assert_eq!(start_trimmed.right, whole.right, "the end head held still");
}
