//! Growing a layer: which of its edges the scale leaves where it was.

use scorsese_compositor::{Frame, Properties};
use scorsese_core::{
    AssetId, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, Origin, OriginX, OriginY,
    PropertyPath,
};

use crate::common::{CENTRE, RED, SIZE, solid};
use crate::{assert_span, at, drawn, scaled, span};

#[test]
fn each_horizontal_origin_holds_its_own_edge_still() {
    // Half width on a 64-pixel canvas is 32 pixels of layer: pinned to the
    // left it occupies 0–31, pinned to the right 32–63, and centred the 16–47
    // in the middle, which is what every layer did before the field existed.
    let source = solid(RED);
    let half = (0.5, 1.0);
    let row = |frame: &Frame| span(frame, |x| (x, CENTRE.1));

    let left = scaled(&source, half, at(OriginX::Left, OriginY::Center));
    assert_span(row(&left), (0, 31), "pinned to its left edge");

    let centre = scaled(&source, half, Origin::default());
    assert_span(row(&centre), (16, 47), "about its own centre");

    let right = scaled(&source, half, at(OriginX::Right, OriginY::Center));
    assert_span(row(&right), (32, 63), "pinned to its right edge");
}

#[test]
fn each_vertical_origin_holds_its_own_edge_still() {
    let source = solid(RED);
    let half = (1.0, 0.5);
    let column = |frame: &Frame| span(frame, |y| (CENTRE.0, y));

    let top = scaled(&source, half, at(OriginX::Center, OriginY::Top));
    assert_span(column(&top), (0, 31), "pinned to its top edge");

    let bottom = scaled(&source, half, at(OriginX::Center, OriginY::Bottom));
    assert_span(column(&bottom), (32, 63), "pinned to its bottom edge");
}

#[test]
fn a_left_origin_holds_the_left_edge_at_every_scale() {
    // Several sizes rather than one, because a pivot half a box out lands on
    // the right answer for exactly one of them.
    let source = solid(RED);
    for scale in [0.125, 0.25, 0.5, 0.75, 1.0] {
        let canvas = scaled(&source, (scale, 1.0), at(OriginX::Left, OriginY::Center));
        let width = (f64::from(SIZE) * scale).round() as u32;
        assert_span(
            span(&canvas, |x| (x, CENTRE.1)),
            (0, width - 1),
            &format!("scaled to {scale}"),
        );
    }
}

#[test]
fn a_position_moves_a_pivoted_layer_exactly_as_it_moves_a_centred_one() {
    // Position is applied *after* the pivot, so an origin cannot change what a
    // move is worth. A quarter of the canvas is 16 pixels across and 16 down
    // whichever corner the half-size layer was grown from, and that is what
    // makes the field free to set on a clip that is only being placed. Both
    // axes, because a sign is per-axis and a test of one proves nothing of the
    // other.
    let source = solid(RED);
    let moved = |origin| {
        drawn(
            &source,
            Properties {
                scale: (0.5, 0.5),
                position: (0.25, 0.25),
                ..Properties::default()
            },
            origin,
        )
    };

    // Grown about the middle the layer covers 16–47, and the move takes it to
    // 32–63 on both axes. Each line is sampled through the middle of where the
    // layer is expected to be, so a wrong answer is a miss rather than a
    // fringe.
    let centre = moved(Origin::default());
    assert_span(
        span(&centre, |x| (x, 48)),
        (32, 63),
        "centred, moved across",
    );
    assert_span(span(&centre, |y| (48, y)), (32, 63), "centred, moved down");

    // Grown from its top-left corner it covers 0–31, and the same move takes
    // it to 16–47 — the same sixteen pixels, on both axes.
    let corner = moved(at(OriginX::Left, OriginY::Top));
    assert_span(
        span(&corner, |x| (x, 32)),
        (16, 47),
        "cornered, moved across",
    );
    assert_span(span(&corner, |y| (32, y)), (16, 47), "cornered, moved down");
}

/// A bar that fills from its left edge over `duration` frames, on a curve.
fn filling(duration: u64) -> Clip {
    let mut clip = Clip::new(
        ClipId::new("c-bar"),
        AssetId::new("bar"),
        Frames::ZERO,
        Frames(duration),
    );
    clip.origin = at(OriginX::Left, OriginY::Center);
    let ends = |t, value| Keyframe {
        t,
        value,
        easing: Easing::EaseOut,
    };
    clip.keyframes.push(KeyframeTrack::new(
        PropertyPath::new("transform.scale.x"),
        vec![ends(Frames::ZERO, 0.0), ends(Frames(duration), 1.0)],
    ));
    clip
}

#[test]
fn an_eased_fill_holds_its_left_edge_where_two_tracks_would_not() {
    // The case the workaround gets wrong, and the reason the field exists.
    // Holding a left edge still by hand is a second track at `(s - 1) / 2`,
    // which only tracks the scale while the scale is linear in time; put an
    // `ease_out` on it and the bar slides while it grows. A pivot has nothing
    // to come apart from, so the edge is exact at every frame of the curve.
    let source = solid(RED);
    let clip = filling(60);
    let halfway = Properties::at(&clip, Frames(30)).scale.0;
    assert!(
        halfway > 0.7,
        "the curve has to be doing something, or this proves nothing: {halfway}"
    );
    for t in [6, 15, 30, 45, 60] {
        let properties = Properties::at(&clip, Frames(t));
        let canvas = drawn(&source, properties, clip.origin);
        let width = (f64::from(SIZE) * properties.scale.0).round() as u32;
        assert_span(
            span(&canvas, |x| (x, CENTRE.1)),
            (0, width - 1),
            &format!("at frame {t}, scaled to {}", properties.scale.0),
        );
    }
}
