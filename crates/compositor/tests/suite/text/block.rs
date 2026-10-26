//! Where a block would be set, asked without setting it.

use scorsese_compositor::text::{self, Band, Font};
use scorsese_compositor::{Area, Resolution};
use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba};

use crate::text::ink::{HEIGHT, WIDTH, style};

fn block(content: &str, max_width: f32) -> Area {
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a legal raster");
    placed(
        content,
        max_width,
        Anchor::default(),
        Band::whole(resolution),
    )
}

fn placed(content: &str, max_width: f32, anchor: Anchor, band: Band) -> Area {
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a legal raster");
    let style = text::Style {
        max_width,
        anchor,
        ..style(20.0, Rgba::WHITE)
    };
    text::block_in(content, Font::sans(), &style, band, resolution)
}

/// A width that is not a width — none, negative, endless or not a number —
/// is the frame's, rather than a block of no width or one off the frame.
#[test]
fn a_width_that_is_not_a_width_is_the_frames() {
    for max_width in [0.0, -40.0, f32::INFINITY, f32::NAN] {
        let found = block("Ship it", max_width);
        assert_eq!(
            (found.left, found.width),
            (0.0, WIDTH as f32),
            "{max_width}"
        );
    }
}

/// A block is as tall as its lines, each one line height: a 20-pixel style
/// at the default 1.25 is 25 pixels a line, so two lines are fifty.
#[test]
fn a_block_is_as_tall_as_its_lines() {
    assert_eq!(block("Ship it", 300.0).height, 25.0);
    assert_eq!(block("Ship\nit", 300.0).height, 50.0);
}

/// The anchor places the block in its band: one line, 100 wide, in a band
/// 100 tall starting 40 down a 320 × 180 frame. Centred is 110 across and
/// 37.5 into the band; bottom-right is flush with the frame's right edge and
/// the band's foot.
#[test]
fn the_anchor_places_the_block_in_its_band() {
    let band = Band {
        top: 40.0,
        height: 100.0,
    };
    let at = |x, y| {
        let found = placed("Ship it", 100.0, Anchor { x, y }, band);
        (found.left, found.top)
    };
    assert_eq!(at(AnchorX::Center, AnchorY::Center), (110.0, 77.5));
    assert_eq!(at(AnchorX::Right, AnchorY::Bottom), (220.0, 115.0));
}

/// Under 1.0 the lines are set tighter than solid, as asked, down to half the
/// size (#1090): a 20-pixel style at 0.8 is 16 pixels a line, and one asking
/// for less than 0.5 gets 10.
#[test]
fn a_line_height_under_the_size_is_honoured_down_to_half_of_it() {
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a legal raster");
    let tall = |line_height: f32| {
        let style = text::Style {
            line_height,
            max_width: 300.0,
            ..style(20.0, Rgba::WHITE)
        };
        text::block_in(
            "Ship\nit",
            Font::sans(),
            &style,
            Band::whole(resolution),
            resolution,
        )
        .height
    };
    assert_eq!(tall(16.0), 32.0);
    assert_eq!(tall(4.0), 20.0);
    assert_eq!(tall(f32::NAN), 20.0);
}

/// What setting the block found out besides where it is (#1096): a band one
/// line tall holding two lines cuts one, and a short line's words reach only
/// as far as the line, centred in its block.
#[test]
fn a_setting_counts_the_lines_cut_and_measures_the_words_not_the_block() {
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a legal raster");
    let style = text::Style {
        max_width: 300.0,
        ..style(20.0, Rgba::WHITE)
    };
    let set = |content: &str, height: f32| {
        let band = Band { top: 0.0, height };
        text::setting_in(content, Font::sans(), &style, band, resolution)
    };
    let cut = set("Ship\nit", 25.0);
    assert_eq!(cut.overset, Some(text::Overset { set: 2, shown: 1 }));
    let whole = set("Ship\nit", HEIGHT as f32);
    assert_eq!(whole.overset, None);
    assert!(whole.ink.width > 0.0 && whole.ink.width < whole.block.width);
    let middle = |area: scorsese_compositor::Area| area.left + area.width / 2.0;
    assert!((middle(whole.ink) - middle(whole.block)).abs() < 0.01);
}
