//! A contact sheet's timestamps: that they arrived, and where.
//!
//! The sheet is the picture an assistant looks at footage through, and the
//! moment written on each cell is the entire point of it — a grid of frames
//! with no times on it says *there is footage* and nothing else. Nothing here
//! asserted that the stamping happened at all, so the whole of it could be
//! removed and the suite stayed green.
//!
//! Every cell is drawn white, so the two things a stamp puts down separate by
//! brightness alone: the near-black panel darkens the strip it covers, and the
//! words come back to white on top of it. Anything that is still white outside
//! the strip is a cell nobody drew on, which is what makes "and not above it"
//! measurable rather than assumed.
//!
//! The rows are the arithmetic rather than a measurement: `LABEL_HEIGHT` is
//! `0.13` of the cell, so a 200-pixel cell gives a strip of exactly 26 and the
//! panel runs from row 174 to the bottom edge.

use scorsese_compositor::sheet::{self, Cell};
use scorsese_compositor::text::Font;
use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resolution};
use scorsese_core::Rgba;

/// A cell size whose label strip lands on whole rows.
const WIDTH: u32 = 240;
const HEIGHT: u32 = 200;

/// The first row of the label strip: `200 - 200 * 0.13`.
const STRIP_TOP: u32 = 174;

/// A pixel this bright is either an untouched cell or a word on the panel; the
/// panel itself is black at `0xb4` over white, which lands near `0x4b`.
const BRIGHT: u8 = 0xd0;

/// One cell: a white picture, and the moment it came from.
fn cell(label: &str) -> Cell {
    let mut frame = Frame::black(Resolution::new(WIDTH, HEIGHT).expect("a legal raster"));
    frame.fill(Rgba::WHITE);
    Cell {
        frame,
        label: label.to_owned(),
    }
}

fn tiled(labels: &[&str]) -> Frame {
    sheet::tile(
        labels.iter().copied().map(cell).collect(),
        Font::sans(),
        false,
    )
    .expect("a sheet of same-sized cells tiles")
}

/// The red channel at one pixel, which is brightness here: everything drawn is
/// grey.
fn brightness(frame: &Frame, x: u32, y: u32) -> u8 {
    let width = frame.resolution().width() as usize;
    frame.bytes()[(y as usize * width + x as usize) * BYTES_PER_PIXEL]
}

/// The columns of `column`'s cell, as `x` within it.
fn across(column: u32) -> std::ops::Range<u32> {
    (column * WIDTH)..((column + 1) * WIDTH)
}

/// The first and last row of `column`'s cell that the stamp touched at all —
/// anything left of the white the cell was drawn as.
fn touched(frame: &Frame, column: u32) -> Option<(u32, u32)> {
    let drawn = |y| across(column).any(|x| brightness(frame, x, y) != u8::MAX);
    let first = (0..HEIGHT).find(|&y| drawn(y))?;
    Some((first, (0..HEIGHT).rfind(|&y| drawn(y))?))
}

/// How many pixels of the strip came back bright — the words — and where the
/// middle of them sits, measured from the left edge of `column`'s cell.
fn words(frame: &Frame, column: u32) -> (usize, f64) {
    let mut count = 0;
    let mut sum = 0.0;
    for y in STRIP_TOP..HEIGHT {
        for x in across(column) {
            if brightness(frame, x, y) > BRIGHT {
                count += 1;
                sum += f64::from(x - column * WIDTH);
            }
        }
    }
    (count, sum / count.max(1) as f64)
}

/// The stamp reaches the strip and stops at the top of it, and the words it
/// writes are centred across the cell.
#[test]
fn a_cells_moment_is_written_in_the_strip_and_not_above_it() {
    let frame = tiled(&["0:00"]);

    assert_eq!(
        touched(&frame, 0),
        Some((STRIP_TOP, HEIGHT - 1)),
        "the strip is drawn on from its top row to the bottom edge, and nothing above it is"
    );
    let (count, middle) = words(&frame, 0);
    assert!(
        count > 20,
        "the moment is written on the panel, found {count}"
    );
    assert!(
        (middle - f64::from(WIDTH) / 2.0).abs() <= 2.0,
        "a centred moment sits over the middle of its cell, found {middle}"
    );
}

/// Each cell is stamped with its own moment, in its own cell: two cells given
/// different text carry different amounts of ink, each centred on the cell it
/// belongs to. A sheet that stamped only its first cell, or wrote one moment
/// across the whole picture, fails both halves.
#[test]
fn two_cells_are_stamped_with_their_own_moments() {
    let frame = tiled(&["0:00", "0:00:00"]);
    let (short, first) = words(&frame, 0);
    let (long, second) = words(&frame, 1);

    assert!(
        short > 20 && long > short,
        "`0:00` is the shorter of the two moments, found {short} against {long}"
    );
    for (middle, cell) in [(first, 0), (second, 1)] {
        assert!(
            (middle - f64::from(WIDTH) / 2.0).abs() <= 2.0,
            "cell {cell}'s moment is centred on cell {cell}, found {middle}"
        );
    }
}
