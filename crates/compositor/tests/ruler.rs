//! The ruler's two passes, measured rather than assumed.
//!
//! `grid::draw` lays a dark edging down and then a pale rule over the top of it,
//! and the whole reason there are two is that the pale core has to sit *inside* a
//! dark outline: a white line vanishes on a white shot and a black one vanishes
//! on a night exterior. That only holds while the edging is genuinely thicker
//! than the rule it outlines, and every one of those thicknesses is a multiple of
//! one unit taken from the raster's shorter side. Nothing asserted either fact,
//! so every multiplier could become an addition or a division and the ruler still
//! came out as lines on a frame.
//!
//! **The two passes are told apart by colour, not by alpha.** They overlap by
//! design, so `tests/common/extent.rs` — how much ink there is, and where all of
//! it is — cannot say which pass laid a pixel down; that is a question its
//! measurements are the wrong shape for rather than a gap in them. Over a
//! transparent frame the edging alone comes out black and the rule over it comes
//! out pale, so across a row: a span of *any* ink is the edging's width, and the
//! span of *pale* ink inside it is the rule's.
//!
//! Every raster here is sized so the ruler's rectangles land on whole pixels,
//! which is what makes a measured span the geometry rather than the geometry plus
//! whatever an anti-aliased edge added. A pixel of slack is allowed anyway, and
//! no more — the same pixel `extent.rs` allows, and far less than any of the
//! arithmetic this is here to object to.

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resolution, grid};

/// How many divisions the ruler cuts the frame into — its own `DIVISIONS`.
const DIVISIONS: u32 = 10;

/// A 4K raster's shorter side. 2160 is exactly four times the 540 `draw`
/// measures its unit against, so a unit is 4 whole pixels and the weights come
/// out 4, 12, 12 and 20 — each far enough from what an addition or a division
/// would give that a pixel of slack cannot hide the difference.
const FOUR_K: u32 = 2160;

/// One unit at [`FOUR_K`], in pixels.
const UNIT: f64 = 4.0;

/// A contact sheet's cell: under 540 on its shorter side, so the unit is held at
/// 1 by the floor and the rule is a single pixel inside three of edging. This is
/// the size the two passes are closest at, and the only size at which an edging
/// thinner than its rule disappears under it outright.
///
/// **Odd on purpose.** At a unit of 1 the weights are odd numbers, so a line
/// centred on a whole pixel straddles two of them and every span picks up an
/// anti-aliased column at each end; centred on a half pixel it does not. An odd
/// raster is what puts these gridlines on halves, and a cell of a contact sheet
/// is not encoded, which is what [`Resolution::source`] is for.
const CELL: u32 = 535;

/// A frame small enough that the unit would come out *below* a pixel: 105 on its
/// shorter side is a fifth of the 540 `draw` references, so without the floor
/// under the unit the rule would be a fifth of a pixel of ink. Wide, so the
/// gridline it is read at still stands well clear of its neighbours.
const TINY: (u32, u32) = (955, 105);

/// Where the `0.3` and `0.5` gridlines sit on each raster, and a row that cuts
/// through them clear of every horizontal rule and every label. [`TINY`] names
/// its minor line only, which is the one the floor is read off.
const FOUR_K_LINES: (u32, u32, u32) = (648, 1080, 756);
const CELL_LINES: (u32, u32, u32) = (160, 267, 70);
const TINY_LINE: (u32, u32) = (286, 25);

/// Columns holding the `0.5` label on [`LABEL_WIDTH`] and nothing else — past
/// the heavy rule under it, short of the `0.6` label beside it.
const LABEL_LEFT: u32 = 483;
const LABEL_RIGHT: u32 = 540;

/// A raster wide enough that the top labels stand well apart, so the window
/// above holds one of them alone whatever the frame's height.
const LABEL_WIDTH: u32 = 960;

/// Red and alpha at one pixel — the two channels that separate the passes.
fn ink(frame: &Frame, x: u32, y: u32) -> (u8, u8) {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 3])
}

/// A ruled frame, drawn over transparency so that "there is ink here" means the
/// ruler is what put it there.
fn ruled(width: u32, height: u32) -> Frame {
    let resolution = Resolution::source(width, height).expect("a legal raster");
    let mut frame = Frame::black(resolution);
    frame.fill_transparent();
    grid::draw(&mut frame);
    frame
}

/// How wide each pass came out at one vertical gridline, measured across `row`
/// inside a window that holds that line and nothing else.
struct Weight {
    /// Columns holding any ink at all: the dark edging, the outermost pass —
    /// or, where the arithmetic went wrong and the rule came out wider, the
    /// rule, which is exactly the failure being looked for.
    edging: u32,
    /// How many of those columns came out pale: the rule drawn inside it.
    rule: u32,
}

fn weight(frame: &Frame, at: u32, row: u32, reach: u32) -> Weight {
    let mut found = Weight { edging: 0, rule: 0 };
    for x in (at - reach)..(at + reach) {
        let (red, alpha) = ink(frame, x, row);
        if alpha > 0 {
            found.edging += 1;
        }
        // Halfway is nowhere near either pass: the edging alone is black, and
        // the rule over it comes out past 0xe0 whatever it is drawn across.
        if red > 128 {
            found.rule += 1;
        }
    }
    found
}

#[track_caller]
fn assert_weight(found: u32, expected: f64, what: &str) {
    let found = f64::from(found);
    assert!(
        (found - expected).abs() <= 1.0,
        "{what} should be {expected} pixels wide, found {found}"
    );
}

/// The row the top labels' baseline sits on, as the lowest row holding ink in a
/// window that contains one label and nothing else.
///
/// It is `1.05` em below the top edge plus the drop shadow, so it moves with the
/// label size and is a measure of it. The ceiling keeps the `0.1` rule out; the
/// rules clamped to the top edge are above every label and cannot reach a lowest
/// row.
fn label_foot(frame: &Frame) -> u32 {
    let ceiling = frame.resolution().height() / DIVISIONS - 4;
    (0..ceiling)
        .filter(|&y| (LABEL_LEFT..=LABEL_RIGHT).any(|x| ink(frame, x, y).1 > 0))
        .max()
        .expect("the labels are drawn")
}

/// The weights are 1 and 3 units of rule inside 3 and 5 of edging. Stated as
/// multiples of one unit because that is what `draw` computes: the numbers below
/// are that arithmetic at [`FOUR_K`] rather than anything read off a run.
#[test]
fn every_weight_is_its_own_multiple_of_one_unit() {
    let frame = ruled(FOUR_K, FOUR_K);
    let (minor_at, major_at, row) = FOUR_K_LINES;
    let minor = weight(&frame, minor_at, row, 24);
    let major = weight(&frame, major_at, row, 24);

    assert_weight(minor.rule, UNIT, "a minor rule");
    assert_weight(minor.edging, 3.0 * UNIT, "the edging under a minor rule");
    assert_weight(major.rule, 3.0 * UNIT, "the heavy rule");
    assert_weight(major.edging, 5.0 * UNIT, "the edging under the heavy rule");
}

/// And the property those multiples exist for, at the one size it is tightest:
/// the edging reaches past the rule it outlines, at both weights.
#[test]
fn the_edging_reaches_past_the_rule_on_a_contact_sheets_cell() {
    let frame = ruled(CELL, CELL);
    let (minor_at, major_at, row) = CELL_LINES;
    let minor = weight(&frame, minor_at, row, 12);
    let major = weight(&frame, major_at, row, 12);

    assert!(
        minor.edging > minor.rule,
        "a minor rule {} wide should be outlined by an edging wider than it, found {}",
        minor.rule,
        minor.edging
    );
    assert!(
        major.edging > major.rule,
        "the heavy rule {} wide should be outlined by an edging wider than it, found {}",
        major.rule,
        major.edging
    );
    assert!(
        major.rule > minor.rule,
        "the heavy rule should be wider than a minor one, found {} against {}",
        major.rule,
        minor.rule
    );
}

/// The other floor in `draw`, the one under the unit itself: however small the
/// frame, the rule is a whole pixel of pale inside three of edging. A fraction of
/// a pixel is not a thin line — it is one the rasteriser spreads over two columns
/// at a fraction of the ink each, which comes out as a smudge too faint to read a
/// coordinate off, and the pale core stops being pale at all.
#[test]
fn a_rule_is_never_thinner_than_a_pixel() {
    let (width, height) = TINY;
    let (at, row) = TINY_LINE;
    let minor = weight(&ruled(width, height), at, row, 12);
    assert!(
        minor.rule >= 1 && minor.edging >= 3,
        "a {width}x{height} frame should still be ruled a pixel of rule inside \
         three of edging, found {} inside {}",
        minor.rule,
        minor.edging
    );
}

/// `LABEL_FLOOR` is what keeps a contact sheet's labels readable: the fraction
/// alone puts them at 9 pixels on a 200-pixel cell, so the floor holds them at
/// 13 — the same size a 290-pixel frame asks for on its own. Both frames are
/// therefore labelled identically, and without the floor the smaller one's
/// baseline would sit four pixels higher.
#[test]
fn labels_stop_shrinking_with_the_frame() {
    let floored = label_foot(&ruled(LABEL_WIDTH, 200));
    let asked_for = label_foot(&ruled(LABEL_WIDTH, 290));
    assert!(
        floored.abs_diff(asked_for) <= 1,
        "labels held at the floor should sit where a 13-pixel em does, \
         found baselines at {floored} and {asked_for}"
    );
}
