//! The two colours: what a fill covers, and where a border sits.

use scorsese_compositor::shape::{Border, Figure, draw};

use crate::{BLUE, at, boxed, centred, clear, filled, frame};

/// A border straddles the outline — half in, half out — so the pixels either
/// side of the stated edge are both border, and the fill is what is found well
/// inside it. A border drawn wholly inside would make a shape smaller than the
/// size the document gave it, which nothing downstream could correct for.
#[test]
fn a_border_straddles_the_edge_it_bounds() {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            border: Some(Border {
                color: BLUE,
                width: 8.0,
            }),
            ..boxed((100.0, 100.0), centred())
        },
    );

    let edge = 50;
    assert_eq!(at(&frame, 100, edge - 2).2, 0xff, "outside the stated edge");
    assert_eq!(at(&frame, 100, edge + 2).2, 0xff, "inside it");
    assert!(filled(&frame, 100, 100), "the fill, well clear of both");
    assert!(
        clear(&frame, 100, edge - 8),
        "beyond the border's outer half"
    );
}

/// A shape with no fill is the callout case: a border, and the footage showing
/// through the middle of it.
#[test]
fn a_bordered_shape_with_no_fill_is_hollow() {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            fill: None,
            border: Some(Border {
                color: BLUE,
                width: 6.0,
            }),
            ..boxed((100.0, 100.0), centred())
        },
    );

    assert_eq!(at(&frame, 100, 50).2, 0xff, "the border is drawn");
    assert!(clear(&frame, 100, 100), "and the middle is see-through");
}

/// The border goes on after the fill, so a translucent interior does not wash
/// over the line bounding it.
#[test]
fn a_border_over_a_translucent_fill_keeps_its_own_colour() {
    let mut frame = frame();
    let mut figure = boxed((100.0, 100.0), centred());
    figure.fill = Some(scorsese_core::Rgba::new(0xff, 0x00, 0x00, 0x40));
    figure.border = Some(Border {
        color: BLUE,
        width: 10.0,
    });
    draw(&mut frame, &figure);

    let (r, _, b, a) = at(&frame, 100, 50);
    assert_eq!((b, a), (0xff, 0xff), "the border is solid blue");
    assert_eq!(r, 0x00, "with no red bleeding through from under it");
}
