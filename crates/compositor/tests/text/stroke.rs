//! The rim a caption carries: where its ink lands, and what it leaves alone.
//!
//! Glyph shapes are the golden fixtures' business. What is measured here is the
//! one thing a picture would not say out loud — that the rim is *added outside*
//! the letterform rather than straddling its outline. An `I` in the shipped
//! sans is a plain rectangular stem, so a row through the middle of one is a
//! run of rim, a run of fill, and a run of rim, and each of those runs is a
//! number a test can hold to.

use scorsese_compositor::text::{self, Edge, Font, Style};
use scorsese_compositor::{BYTES_PER_PIXEL, Frame};
use scorsese_core::Rgba;

use crate::ink::{self, HEIGHT, canvas, style};

/// Black on white, which is the caption case and the pair furthest apart, so a
/// pixel that is one of them cannot be mistaken for the other.
const RIM: Rgba = Rgba::opaque(0, 0, 0);

/// Big enough that the stem is many pixels across and the rim is many more, so
/// an anti-aliased boundary is a rounding error rather than the measurement.
const SIZE: f32 = 100.0;
const THICK: f32 = 6.0;

fn drawn(edge: Option<Edge>) -> Frame {
    let mut frame = canvas();
    let style = Style {
        edge,
        ..style(SIZE, Rgba::WHITE)
    };
    text::draw(&mut frame, "I", Font::sans(), &style);
    frame
}

fn rimmed() -> Frame {
    drawn(Some(Edge {
        color: RIM,
        width: THICK,
    }))
}

/// How many pixels of the middle row are within a channel of `color`, opaque.
///
/// A channel of slack rather than an exact match: the row crosses two
/// anti-aliased boundaries, and a pixel one level off is still that colour.
fn run(frame: &Frame, color: Rgba) -> u32 {
    let y = HEIGHT / 2;
    (0..frame.resolution().width())
        .filter(|x| {
            let (r, g, b, a) = ink::pixel(frame, *x, y);
            let [want_r, want_g, want_b, _] = color.channels();
            a == 255
                && r.abs_diff(want_r) <= 1
                && g.abs_diff(want_g) <= 1
                && b.abs_diff(want_b) <= 1
        })
        .count() as u32
}

/// **The measurement the whole choice rests on.** A rim behind the fill leaves
/// the stem exactly as wide as it was; a rim centred on the outline — what a
/// shape's border does — would eat `THICK` pixels out of it, three from each
/// side, and a counter would close the same way at caption sizes.
#[test]
fn the_stem_is_no_thinner_for_having_a_rim() {
    let bare = run(&drawn(None), Rgba::WHITE);
    let rimmed = run(&rimmed(), Rgba::WHITE);
    assert!(bare > 4, "the stem should be several pixels across: {bare}");
    assert!(
        rimmed.abs_diff(bare) <= 1,
        "a rim behind the fill leaves the stem alone: {bare} bare, {rimmed} rimmed"
    );
}

/// And the rim is as thick as it was asked to be, on both sides of the stem.
#[test]
fn the_rim_reaches_its_width_outside_the_letter() {
    let frame = rimmed();
    let rim = run(&frame, RIM);
    let want = (THICK * 2.0) as u32;
    assert!(
        rim.abs_diff(want) <= 2,
        "a {THICK}px rim either side of the stem is {want} pixels of the row; found {rim}"
    );
}

/// The block grows outward by the width, which is the same claim measured
/// against the ink's own box rather than one row of it.
#[test]
fn the_ink_grows_outward_by_the_width() {
    let bare = ink::bounds(&drawn(None)).expect("a letter was drawn");
    let rimmed = ink::bounds(&rimmed()).expect("a letter was drawn");
    let grew = THICK as u32;
    for (side, (bare, rimmed), sign) in [
        ("left", (bare.0, rimmed.0), -1i64),
        ("top", (bare.1, rimmed.1), -1),
        ("right", (bare.2, rimmed.2), 1),
        ("bottom", (bare.3, rimmed.3), 1),
    ] {
        let moved = i64::from(rimmed) - i64::from(bare);
        assert!(
            (moved - sign * i64::from(grew)).abs() <= 1,
            "the {side} edge should move {grew} outward; it moved {moved}"
        );
    }
}

/// A rim with no thickness is not a rim, and must not be a faint one either:
/// the frame has to come out the same bytes as one that never asked for an
/// edge, or `stroke_width: 0` would be a caption nobody can explain.
#[test]
fn a_rim_of_no_width_draws_nothing_at_all() {
    let none = drawn(None);
    let zero = drawn(Some(Edge {
        color: RIM,
        width: 0.0,
    }));
    assert_eq!(none.bytes(), zero.bytes());
}

/// A see-through rim is composited like anything else here, so what lands over
/// a transparent layer is the colour at its own alpha rather than a flat one.
#[test]
fn a_translucent_rim_keeps_its_alpha() {
    let frame = drawn(Some(Edge {
        color: Rgba::new(0, 0, 0, 128),
        width: THICK,
    }));
    let alphas: Vec<u8> = frame
        .bytes()
        .chunks_exact(BYTES_PER_PIXEL)
        .map(|pixel| pixel[3])
        .filter(|alpha| (120..=136).contains(alpha))
        .collect();
    assert!(
        alphas.len() > 20,
        "a half-transparent rim leaves half-transparent pixels; found {}",
        alphas.len()
    );
}

/// **A colour glyph takes no rim.** A rim is a stroke of the path being filled,
/// and an emoji is not that path — it is a layered drawing in its own colours,
/// with no single outline to grow anything off. So a fire asked for a rim comes
/// out the same bytes as a fire that never was, while the letters beside it are
/// still rimmed: the caption keeps its legibility and the drawing keeps its
/// shape, rather than the fire acquiring a sticker border.
#[test]
fn a_colour_glyph_is_drawn_as_itself_rather_than_rimmed() {
    let set = |content: &str, edge| {
        let mut frame = canvas();
        let style = Style {
            edge,
            ..style(SIZE, Rgba::WHITE)
        };
        text::draw(&mut frame, content, Font::sans(), &style);
        frame
    };
    let thick = Some(Edge {
        color: RIM,
        width: THICK,
    });
    let bare = set("🔥", None);
    assert!(ink::bounds(&bare).is_some(), "the fire was drawn");
    assert_eq!(
        bare.bytes(),
        set("🔥", thick).bytes(),
        "an emoji has no letterform to grow a rim off, so a rim changes nothing"
    );
    // And the same style on the same line still rims what it can: this is the
    // colour glyph being left alone, not the edge being dropped.
    assert!(
        run(&set("I🔥", thick), RIM) > 0,
        "the letter beside the fire still carries its rim"
    );
}
