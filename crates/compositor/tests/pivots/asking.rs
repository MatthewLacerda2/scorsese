//! Asking where a pivoted layer's edge went, and finding the ink there.
//!
//! `on_canvas` is what an attached arrow asks; the draw is what a viewer sees.
//! They go through one matrix on purpose — a second implementation would drift
//! from the first, and the drift would show as an arrow missing the box it is
//! attached to — so a pivot the query ignored is exactly the bug this rules
//! out. It is asserted here rather than only from `crates/render`, because an
//! assertion in another crate is one no test of this one makes.

use scorsese_compositor::{Area, Properties, on_canvas};
use scorsese_core::{Anchor, Origin, OriginX, OriginY};

use crate::common::{CENTRE, RED, raster, solid};
use crate::{at, scaled, span};

/// Where the layer's own left and right edges land on the canvas, asked of the
/// compositor rather than measured off it.
fn asked(scale: (f64, f64), origin: Origin) -> (f32, f32) {
    let properties = Properties {
        scale,
        ..Properties::default()
    };
    let whole = Area::whole(raster());
    let edge = |across| {
        on_canvas(
            &properties,
            Anchor::default(),
            origin,
            raster(),
            raster(),
            whole.at(across, 0.5),
        )
        .0
    };
    (edge(0.0), edge(1.0))
}

#[test]
fn the_query_and_the_picture_agree_about_where_a_pivoted_edge_went() {
    let source = solid(RED);
    let half = (0.5, 1.0);
    for origin in [
        Origin::default(),
        at(OriginX::Left, OriginY::Center),
        at(OriginX::Right, OriginY::Center),
    ] {
        let (left, right) = asked(half, origin);
        // The measured span is inclusive of its last inked column, where the
        // asked-for right edge is the boundary just past it.
        let (first, last) = span(&scaled(&source, half, origin), |x| (x, CENTRE.1));
        let close = |asked: f32, drawn: u32| (asked - drawn as f32).abs() <= 1.0;
        assert!(
            close(left, first) && close(right, last + 1),
            "{origin:?}: the query says {left}–{right}, the picture {first}–{}",
            last + 1
        );
    }
}
