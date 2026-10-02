//! Where a block would be set, asked without setting it.

use scorsese_compositor::text::{self, Band, Font};
use scorsese_compositor::{Area, Resolution};
use scorsese_core::Rgba;

use crate::ink::{HEIGHT, WIDTH, style};

fn block(content: &str, max_width: f32) -> Area {
    let resolution = Resolution::new(WIDTH, HEIGHT).expect("a legal raster");
    let style = text::Style {
        max_width,
        ..style(20.0, Rgba::WHITE)
    };
    text::block_in(
        content,
        Font::sans(),
        &style,
        Band::whole(resolution),
        resolution,
    )
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
