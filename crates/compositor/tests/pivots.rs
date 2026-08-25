//! What a scale and a turn happen *about*: `origin`.
//!
//! These measure where an edge landed rather than sampling a pixel inside the
//! layer, because where the edge landed is the entire question — the middle of
//! a bar is red whichever end of it the pivot was on.

mod common;

use scorsese_compositor::{Frame, Layer, Properties};
use scorsese_core::{
    AssetId, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, Origin, OriginX, OriginY,
    PropertyPath,
};

use common::{BLACK, CENTRE, RED, SIZE, assert_pixel, composited, pixel, solid, solid_of};

fn at(x: OriginX, y: OriginY) -> Origin {
    Origin { x, y }
}

/// One layer drawn with `properties` about `origin`, and nothing else on the
/// canvas.
fn drawn(source: &Frame, properties: Properties, origin: Origin) -> Frame {
    composited(&[Layer {
        properties,
        origin,
        ..Layer::plain(source)
    }])
}

fn scaled(source: &Frame, scale: (f64, f64), origin: Origin) -> Frame {
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
fn span(frame: &Frame, along: impl Fn(u32) -> (u32, u32)) -> (u32, u32) {
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
fn assert_span(found: (u32, u32), expected: (u32, u32), what: &str) {
    let close = |a: u32, b: u32| a.abs_diff(b) <= 1;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1),
        "{what}: expected the layer to span {expected:?}, found {found:?}"
    );
}

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

/// A bar as wide as the canvas and a quarter as tall, resting across the
/// middle. A quarter turn about its own centre stands it upright through the
/// middle; about its left edge it swings away from there entirely.
fn bar() -> Frame {
    solid_of(RED, SIZE, SIZE / 4)
}

#[test]
fn a_turn_happens_about_the_origin_too() {
    let source = bar();
    let quarter = Properties {
        rotation: 90.0,
        ..Properties::default()
    };

    let centred = drawn(&source, quarter, Origin::default());
    assert_pixel(&centred, CENTRE, RED, "a centred turn stays in the middle");

    // Pivoting on the left edge, the bar's near end is 8 pixels either side of
    // where it rested and its far end swings down off the canvas — so what is
    // left is a strip 16 wide, half of it past the left edge.
    let hinged = drawn(&source, quarter, at(OriginX::Left, OriginY::Center));
    assert_pixel(&hinged, CENTRE, BLACK, "a hinged one no longer crosses it");
    assert_span(
        span(&hinged, |x| (x, 40)),
        (0, 7),
        "hinged on its left edge",
    );
    assert_pixel(&hinged, (4, 24), BLACK, "and swung clockwise, not up");
}
