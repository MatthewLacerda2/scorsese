//! What the stacks promise, asserted as pixels on a raster.
//!
//! A file of its own because the module it tests is at the size gate's limit
//! and because these are a set: every one of them drives [`super::Canvas`]
//! through the callbacks a real walk is made of and reads the answer off the
//! frame. Nothing here goes near a font's own `COLR` tree — what a glyph asks
//! for is skrifa's business, and what we do with what it asks is this.

use skrifa::color::{ColorStop, Extend};

use crate::text::font::Font;

use super::*;

/// Big enough that a 100px letter sits well inside it with room to be
/// pushed around.
const RASTER: u32 = 300;
const SIZE: f32 = 100.0;
/// Where the pen is: the left edge of the glyph and the baseline it sits on.
const ORIGIN: (f32, f32) = (40.0, 200.0);

/// An opaque fill in the caption's own colour — palette index `0xFFFF`,
/// which resolves without a `CPAL` at all, so these tests need no palette
/// and the shipped text face will do.
const INK: Brush<'static> = Brush::Solid {
    palette_index: 0xffff,
    alpha: 1.0,
};

/// The raster left by a sequence of the callbacks a walk is made of.
///
/// The glyph id comes back with the canvas because it is read off the same
/// face: `H` is two stems and a bar, which is a shape wide and tall enough
/// for a clip or a translation to visibly move.
fn walked(steps: impl FnOnce(&mut Canvas<'_, '_>, GlyphId)) -> Pixmap {
    let font = Font::sans();
    let face = font.at(SIZE);
    let id = face
        .shape("H", 0)
        .glyphs
        .first()
        .expect("the shipped sans face has an H")
        .id;
    let mut into = Pixmap::new(RASTER, RASTER).expect("a legal raster");
    let mut said = Vec::new();
    steps(
        &mut Canvas::new(&mut into, &face, ORIGIN, Rgba::WHITE, &mut said),
        id,
    );
    into
}

/// How many pixels carry any ink at all.
fn inked(pixmap: &Pixmap) -> usize {
    pixmap.pixels().iter().filter(|p| p.alpha() > 0).count()
}

/// The box the ink occupies, as `(left, top, right, bottom)` inclusive.
fn bounds(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        if pixel.alpha() == 0 {
            continue;
        }
        let (x, y) = (index as u32 % RASTER, index as u32 / RASTER);
        found = Some(match found {
            None => (x, y, x, y),
            Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
        });
    }
    found
}

#[test]
fn a_glyph_lands_at_the_pen_the_right_way_up() {
    // The base transform, stated as where the ink is. A capital sits
    // entirely above its own baseline and starts at the pen, so a flip
    // done twice or not at all puts it below, and an origin dropped puts
    // it at the corner.
    let (left, top, _, bottom) = bounds(&walked(|canvas, id| {
        canvas.fill_glyph(id, None, INK);
    }))
    .expect("the letter was drawn");
    assert!(
        bottom <= ORIGIN.1 as u32 + 1,
        "an `H` sits on its baseline, not through it: it reaches row {bottom}"
    );
    assert!(top < ORIGIN.1 as u32 - 40, "…and well above it, at {top}");
    assert!(left >= ORIGIN.0 as u32, "…and starts at the pen, at {left}");
}

#[test]
fn a_transform_moves_what_is_filled_and_popping_it_puts_it_back() {
    let plain = bounds(&walked(|canvas, id| canvas.fill_glyph(id, None, INK)));
    // Half an em to the right, in the font units the walk speaks.
    let moved = bounds(&walked(|canvas, id| {
        canvas.push_transform(shift(1024.0));
        canvas.fill_glyph(id, None, INK);
    }))
    .expect("the letter was drawn");
    let back = bounds(&walked(|canvas, id| {
        canvas.push_transform(shift(1024.0));
        canvas.pop_transform();
        canvas.fill_glyph(id, None, INK);
    }));
    assert!(
        moved.0 > plain.expect("the letter was drawn").0 + 20,
        "the push moved it right: {moved:?}"
    );
    assert_eq!(back, plain, "and the pop put it back exactly");
}

#[test]
fn the_base_transform_survives_a_pop_it_never_had() {
    // A malformed file can pop more than it pushed. Losing the base would
    // leave the rest of the glyph with no way back to the raster at all,
    // so an unbalanced pop has to be a no-op rather than a shrug.
    let plain = bounds(&walked(|canvas, id| canvas.fill_glyph(id, None, INK)));
    let over_popped = bounds(&walked(|canvas, id| {
        canvas.pop_transform();
        canvas.pop_transform();
        canvas.fill_glyph(id, None, INK);
    }));
    assert_eq!(over_popped, plain);
}

#[test]
fn a_clip_narrows_what_a_fill_reaches_and_popping_it_widens_it_again() {
    // `fill` has no shape of its own — it paints whatever clip is in
    // force, which is what makes the clip stack load-bearing rather than
    // an optimisation.
    let whole = inked(&walked(|canvas, _| canvas.fill(INK)));
    let clipped = inked(&walked(|canvas, id| {
        canvas.push_clip_glyph(id);
        canvas.fill(INK);
    }));
    let popped = inked(&walked(|canvas, id| {
        canvas.push_clip_glyph(id);
        canvas.pop_clip();
        canvas.fill(INK);
    }));
    assert_eq!(
        whole,
        (RASTER * RASTER) as usize,
        "a fill under no clip is the whole raster"
    );
    assert!(clipped > 0 && clipped < whole / 4, "clipped to the letter");
    assert_eq!(popped, whole, "and the pop opened it up again");
}

#[test]
fn a_second_clip_intersects_the_first_rather_than_replacing_it() {
    // The stack holds masks already intersected, so what is in force is
    // *both*. A clip that replaced would let the second one's area through
    // where the first excluded it — which is the difference between a
    // shape's shadow and its highlight.
    let letter = inked(&walked(|canvas, id| {
        canvas.push_clip_glyph(id);
        canvas.fill(INK);
    }));
    let upper = inked(&walked(|canvas, _| {
        canvas.push_clip_box(box_of(0.0, 700.0, 2048.0, 2048.0));
        canvas.fill(INK);
    }));
    let both = inked(&walked(|canvas, id| {
        canvas.push_clip_glyph(id);
        canvas.push_clip_box(box_of(0.0, 700.0, 2048.0, 2048.0));
        canvas.fill(INK);
    }));
    assert!(letter > 0 && upper > 0, "each clip lets something through");
    assert!(
        both < letter && both < upper,
        "and the two together let less through than either: {both} against \
         {letter} and {upper}"
    );
}

/// A clip box in the font units the walk speaks, since that is the only
/// space `push_clip_box` is ever handed one in.
fn box_of(x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> BoundingBox<f32> {
    BoundingBox {
        x_min,
        y_min,
        x_max,
        y_max,
    }
}

/// Font units to pixels for the face these tests draw with.
fn scale() -> f32 {
    Font::sans().at(SIZE).scale()
}

/// The font-unit x that lands on raster column `x` under the base transform.
fn at_column(x: f32) -> f32 {
    (x - ORIGIN.0) / scale()
}

/// The font-unit y that lands on raster row `y` — the flip is here, which is
/// why a box's `y_min` is the *lower* row on the screen.
fn at_row(y: f32) -> f32 {
    (ORIGIN.1 - y) / scale()
}

#[test]
fn a_clip_box_is_the_rectangle_it_names_and_not_one_near_it() {
    // A clip box is the only geometry the walk hands over as four numbers
    // rather than as an outline, and each of its sides is a subtraction — so
    // `x_max - x_min` read as `x_max + x_min` is the same rectangle whenever
    // `x_min` happens to be zero and a wrong one otherwise. This box has
    // neither corner at the origin and the assertion is its exact area, so
    // no arithmetic but the right one lands on it.
    let clipped = walked(|canvas, _| {
        canvas.push_clip_box(box_of(
            at_column(60.0),
            at_row(160.0),
            at_column(160.0),
            at_row(60.0),
        ));
        canvas.fill(INK);
    });
    assert_eq!(bounds(&clipped), Some((60, 60, 159, 159)));
    assert_eq!(inked(&clipped), 100 * 100);
}

#[test]
fn a_layer_reaches_the_frame_only_when_it_is_popped() {
    let held = inked(&walked(|canvas, id| {
        canvas.push_layer(CompositeMode::SrcOver);
        canvas.fill_glyph(id, None, INK);
    }));
    let released = inked(&walked(|canvas, id| {
        canvas.push_layer(CompositeMode::SrcOver);
        canvas.fill_glyph(id, None, INK);
        canvas.pop_layer_with_mode(CompositeMode::SrcOver);
    }));
    assert_eq!(held, 0, "an open layer is not the frame");
    assert!(released > 0, "and popping it is what puts it there");
}

#[test]
fn an_inner_layer_pops_onto_the_outer_one_and_not_onto_the_frame() {
    // Two deep, with only the inner one closed. A painter that always drew
    // to the frame would show the letter here, and every emoji whose
    // layers nest would come out composited in the wrong order.
    let nested = inked(&walked(|canvas, id| {
        canvas.push_layer(CompositeMode::SrcOver);
        canvas.push_layer(CompositeMode::SrcOver);
        canvas.fill_glyph(id, None, INK);
        canvas.pop_layer_with_mode(CompositeMode::SrcOver);
    }));
    assert_eq!(nested, 0);
}

#[test]
fn the_mode_a_layer_is_popped_with_is_the_mode_it_composes_in() {
    // A letter on the frame, then a layer holding the same letter closed
    // with `Clear` — which erases what it is composited over. Any mode
    // quietly read as `SourceOver` would leave the letter standing, and
    // the frame would look entirely reasonable.
    let cleared = inked(&walked(|canvas, id| {
        canvas.fill_glyph(id, None, INK);
        canvas.push_layer(CompositeMode::Clear);
        canvas.fill_glyph(id, None, INK);
        canvas.pop_layer_with_mode(CompositeMode::Clear);
    }));
    let kept = inked(&walked(|canvas, id| {
        canvas.fill_glyph(id, None, INK);
        canvas.push_layer(CompositeMode::SrcOver);
        canvas.fill_glyph(id, None, INK);
        canvas.pop_layer_with_mode(CompositeMode::SrcOver);
    }));
    assert!(kept > 0, "over itself, the letter is still there");
    assert_eq!(cleared, 0, "cleared, it is not");
}

#[test]
fn a_brush_transform_applies_inside_the_one_already_in_force() {
    // A gradient is described in the space its shape is described in, so
    // the brush's own matrix is concatenated onto the one in force rather
    // than used instead of it. Two stops of one colour at two alphas, run
    // across the letter: moving the gradient's own space moves where the
    // fade falls, and a transform that was ignored would not move it at
    // all — while one used *instead* of the base would put the letter's
    // ink somewhere the letter is not.
    let across = |stops: &'static [ColorStop]| Brush::LinearGradient {
        p0: skrifa::raw::types::Point::new(0.0, 0.0),
        p1: skrifa::raw::types::Point::new(1400.0, 0.0),
        color_stops: stops,
        extend: Extend::Pad,
    };
    let faded = weight(&walked(|canvas, id| {
        canvas.fill_glyph(id, None, across(&FADE));
    }));
    // The same gradient moved an em either way, so the letter falls
    // entirely outside it and takes the pad on that side: nothing at the
    // transparent end, everything at the opaque one.
    let after = weight(&walked(|canvas, id| {
        canvas.fill_glyph(id, Some(shift(2048.0)), across(&FADE));
    }));
    let before = weight(&walked(|canvas, id| {
        canvas.fill_glyph(id, Some(shift(-2048.0)), across(&FADE));
    }));
    assert!(faded > 0, "the gradient drew the letter");
    assert_eq!(after, 0, "moved past the letter, the fade leaves nothing");
    assert!(
        before > faded * 5 / 4,
        "moved the other way it leaves the letter solid: {before} against \
         {faded}"
    );
}

/// One colour at full strength fading to nothing, which is a gradient a
/// face with no `CPAL` can still describe: `0xFFFF` is the caption's own
/// colour and needs no palette entry.
const FADE: [ColorStop; 2] = [
    ColorStop {
        offset: 0.0,
        palette_index: 0xffff,
        alpha: 0.0,
    },
    ColorStop {
        offset: 1.0,
        palette_index: 0xffff,
        alpha: 1.0,
    },
];

/// A translation of `by` font units to the right, which is the only shape
/// of transform these tests need.
fn shift(by: f32) -> Paint {
    Paint {
        xx: 1.0,
        yx: 0.0,
        xy: 0.0,
        yy: 1.0,
        dx: by,
        dy: 0.0,
    }
}

/// How much ink there is, alpha included — the measurement a fade moves
/// and a count of touched pixels does not.
fn weight(pixmap: &Pixmap) -> u64 {
    pixmap.pixels().iter().map(|p| u64::from(p.alpha())).sum()
}
