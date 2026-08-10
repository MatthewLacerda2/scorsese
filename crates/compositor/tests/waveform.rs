//! The waveform's reduction: an amplitude to a run of rows, and a peak to a
//! colour.
//!
//! Two numbers per column become a shape somebody reads a fault off, and until
//! now nothing in this crate asserted which shape. The picture is 960×260 with
//! its wave area between rows 36 and 216, so the silence line is row 126 and
//! full scale is 90 pixels either side of it — every row named below is that
//! arithmetic done by hand rather than measured off a render.

use scorsese_compositor::text::Font;
use scorsese_compositor::waveform::{self, COLUMNS, Column, Wave};
use scorsese_compositor::{BYTES_PER_PIXEL, Frame};

/// The rules the wave area is drawn between, and the line silence sits on.
const TOP: u32 = 36;
const MIDDLE: u32 = 126;
const BOTTOM: u32 = 216;

/// A column well clear of the three axis ticks, so nothing else is drawn in it.
const X: u32 = 100;

const BACKGROUND: (u8, u8, u8, u8) = (0x14, 0x16, 0x1a, 255);
const PEAK: (u8, u8, u8, u8) = (0x3f, 0x7d, 0xad, 255);
const BODY: (u8, u8, u8, u8) = (0x9e, 0xd8, 0xff, 255);
const CLIPPED: (u8, u8, u8, u8) = (0xe0, 0x5a, 0x50, 255);
const RULE: (u8, u8, u8, u8) = (0x3a, 0x40, 0x4a, 255);

/// One slice of a file: how loud it got, and how loud it was on average.
fn column(peak: f32, rms: f32) -> Column {
    Column { peak, rms }
}

/// A full-width picture whose column [`X`] is the one asked for, the rest
/// silent.
fn drawn(peak: f32, rms: f32) -> Frame {
    let mut columns = vec![Column::default(); COLUMNS];
    columns[X as usize] = column(peak, rms);
    picture(&columns)
}

fn picture(columns: &[Column]) -> Frame {
    waveform::draw(
        &Wave {
            columns,
            seconds: 4.0,
            caption: "narration",
        },
        Font::sans(),
    )
    .expect("the waveform's own size is a legal raster")
}

/// One pixel as `(r, g, b, a)`.
fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

/// One column per pixel of the width, which is what keeps the picture a
/// summary of the file rather than a resampling of a resampling.
#[test]
fn a_column_of_the_picture_is_a_slice_of_the_file() {
    assert_eq!(COLUMNS, 960);
    let frame = picture(&[]);
    assert_eq!(frame.resolution().width(), 960);
    assert_eq!(frame.resolution().height(), 260);
}

/// Full scale reaches the top rule and stops one row short of the bottom one,
/// so the rule below the wave stays readable underneath it.
#[test]
fn a_full_scale_column_fills_the_wave_area_exactly() {
    let frame = drawn(1.0, 0.0);
    assert_eq!(pixel(&frame, X, TOP - 1), BACKGROUND);
    assert_eq!(pixel(&frame, X, TOP), CLIPPED);
    assert_eq!(pixel(&frame, X, BOTTOM - 1), CLIPPED);
    assert_eq!(pixel(&frame, X, BOTTOM), RULE);
}

/// Half of full scale is 45 of the 90 pixels either side of the silence line:
/// rows 81 to 170, and row 171 is where the picture goes back to being empty.
#[test]
fn an_amplitude_reaches_that_fraction_of_the_half_height() {
    let frame = drawn(0.5, 0.0);
    assert_eq!(pixel(&frame, X, 80), BACKGROUND);
    assert_eq!(pixel(&frame, X, 81), PEAK);
    assert_eq!(pixel(&frame, X, 170), PEAK);
    assert_eq!(pixel(&frame, X, 171), BACKGROUND);
}

/// Full scale is the line between the two colours, and it is `>=` rather than
/// `>`: a column that reached exactly `1.0` clipped.
#[test]
fn only_a_column_that_reached_full_scale_reads_as_clipped() {
    assert_eq!(pixel(&drawn(0.999, 0.0), X, TOP + 1), PEAK);
    assert_eq!(pixel(&drawn(1.0, 0.0), X, TOP + 1), CLIPPED);
}

/// The RMS body is drawn second, so it sits inside the peak envelope rather
/// than beside it: at half the peak's amplitude it takes rows 81 to 170 and
/// the envelope keeps everything outside them.
#[test]
fn the_rms_body_is_drawn_inside_the_peak_envelope() {
    let frame = drawn(1.0, 0.5);
    assert_eq!(pixel(&frame, X, 80), CLIPPED);
    assert_eq!(pixel(&frame, X, 81), BODY);
    assert_eq!(pixel(&frame, X, 170), BODY);
    assert_eq!(pixel(&frame, X, 171), CLIPPED);
}

/// Silence is one pixel on the middle line and not nothing at all, because
/// "quiet here" and "no samples here" are different findings.
#[test]
fn a_silent_column_is_one_pixel_on_the_middle_line() {
    let frame = drawn(0.0, 0.0);
    assert_eq!(pixel(&frame, X, MIDDLE - 1), BACKGROUND);
    assert_eq!(pixel(&frame, X, MIDDLE), BODY);
    assert_eq!(pixel(&frame, X, MIDDLE + 1), BACKGROUND);
}

/// An amplitude outside `0.0..=1.0` is clamped to the end of it rather than
/// drawn off the picture: four times full scale is full scale, and below zero
/// is silence.
#[test]
fn an_amplitude_outside_full_scale_is_clamped_to_it() {
    let over = drawn(4.0, 4.0);
    assert_eq!(pixel(&over, X, TOP - 1), BACKGROUND);
    assert_eq!(pixel(&over, X, TOP), BODY);
    assert_eq!(pixel(&over, X, BOTTOM), RULE);

    let under = drawn(-1.0, -1.0);
    assert_eq!(pixel(&under, X, MIDDLE), BODY);
    assert_eq!(pixel(&under, X, MIDDLE + 1), BACKGROUND);
}

/// Fewer slices than there are pixels are drawn as far as they go. Stretching
/// them across the width would be a claim about a file nobody measured.
#[test]
fn fewer_columns_than_the_width_leave_the_rest_of_it_empty() {
    let frame = picture(&[column(1.0, 1.0); 3]);
    assert_eq!(pixel(&frame, 2, TOP + 1), BODY);
    assert_eq!(pixel(&frame, 500, TOP + 1), BACKGROUND);
    assert_eq!(pixel(&frame, 500, MIDDLE), RULE, "the silence line still");
}

/// And more than there are pixels are dropped rather than squeezed in.
#[test]
fn columns_past_the_width_are_dropped() {
    let frame = picture(&vec![column(1.0, 1.0); COLUMNS + 40]);
    assert_eq!(frame.resolution().width(), COLUMNS as u32);
    assert_eq!(pixel(&frame, COLUMNS as u32 - 1, TOP), BODY);
}
