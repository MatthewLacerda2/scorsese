//! What a scale, a turn and a flip happen *about*: `origin`.
//!
//! These measure where an edge landed rather than sampling a pixel inside the
//! layer, because where the edge landed is the entire question — the middle of
//! a bar is red whichever end of it the pivot was on.

#[path = "../common/mod.rs"]
mod common;

mod asking;
mod scaling;
mod turning;

use scorsese_compositor::{Frame, Layer, Properties};
use scorsese_core::{Origin, OriginX, OriginY};

use common::{SIZE, pixel};

pub(crate) fn at(x: OriginX, y: OriginY) -> Origin {
    Origin { x, y }
}

/// One layer drawn with `properties` about `origin`, and nothing else on the
/// canvas.
pub(crate) fn drawn(source: &Frame, properties: Properties, origin: Origin) -> Frame {
    common::composited(&[Layer {
        properties,
        origin,
        ..Layer::plain(source)
    }])
}

pub(crate) fn scaled(source: &Frame, scale: (f64, f64), origin: Origin) -> Frame {
    drawn(
        source,
        Properties {
            scale,
            ..Properties::default()
        },
        origin,
    )
}

/// The first and last step along a line of the canvas carrying the layer's own
/// colour, where `along` turns a step into a pixel.
pub(crate) fn span(frame: &Frame, along: impl Fn(u32) -> (u32, u32)) -> (u32, u32) {
    let inked: Vec<u32> = (0..SIZE)
        .filter(|&step| {
            let (x, y) = along(step);
            pixel(frame, x, y).0 > 128
        })
        .collect();
    let first = *inked
        .first()
        .expect("the layer drew something on this line");
    let last = *inked.last().expect("and so has a far end");
    (first, last)
}

/// Both edges to within a pixel — what an anti-aliased edge falling on a pixel
/// boundary costs, and far less than any wrong pivot.
#[track_caller]
pub(crate) fn assert_span(found: (u32, u32), expected: (u32, u32), what: &str) {
    let close = |a: u32, b: u32| a.abs_diff(b) <= 1;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1),
        "{what}: expected the layer to span {expected:?}, found {found:?}"
    );
}
