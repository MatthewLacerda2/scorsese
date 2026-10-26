//! A contact sheet's timestamps: that they arrived, where, and what they leave
//! alone.
//!
//! The sheet is the picture an assistant looks at footage through, and the
//! moment written on each cell is the entire point of it — a grid of frames
//! with no times on it says *there is footage* and nothing else. Equally, a
//! label drawn over the frame hides the band a sheet is most often taken to
//! check — captions, lower thirds — which is what #919 was.
//!
//! Every cell is drawn white, so the two things a stamp puts down separate by
//! brightness alone: the dark panel of the strip, and the words in white on top
//! of it. Anything not white inside a cell's picture is a pixel the label took.
//!
//! The rows are the arithmetic rather than a measurement: the strip is `0.13`
//! of the cell's shorter side, so a 240x200 cell gets a strip of exactly 26
//! rows, from row 200 to row 225, under a picture left whole.

use scorsese_compositor::sheet::{self, Cell};
use scorsese_compositor::text::Font;
use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resolution};
use scorsese_core::Rgba;

/// A cell size whose label strip lands on whole rows.
const WIDTH: u32 = 240;
const HEIGHT: u32 = 200;

/// The label strip's rows: `200 * 0.13`.
const STRIP: u32 = 26;

/// A pixel this bright is either an untouched picture or a word on the panel;
/// the panel itself is `0x1c`.
const BRIGHT: u8 = 0xd0;

/// One white picture of `width` by `height`, and the moment it came from.
fn sized(width: u32, height: u32, label: &str) -> Cell {
    let mut frame = Frame::black(Resolution::new(width, height).expect("a legal raster"));
    frame.fill(Rgba::WHITE);
    Cell {
        frame,
        label: label.to_owned(),
    }
}

fn tiled(labels: &[&str]) -> Frame {
    sheet::tile(
        labels
            .iter()
            .map(|label| sized(WIDTH, HEIGHT, label))
            .collect(),
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

/// How many pixels of a strip came back bright — the words — and where the
/// middle of them sits, measured from the strip's left edge.
fn words(frame: &Frame, left: u32, width: u32, rows: std::ops::Range<u32>) -> (usize, f64) {
    let mut count = 0;
    let mut sum = 0.0;
    for y in rows {
        for x in left..left + width {
            if brightness(frame, x, y) > BRIGHT {
                count += 1;
                sum += f64::from(x - left);
            }
        }
    }
    (count, sum / count.max(1) as f64)
}

/// The label gets rows of its own under the picture: every pixel of the
/// picture comes back as it went in, and the moment is written, centred, in
/// the strip below it.
#[test]
fn a_cells_moment_is_written_under_its_picture_and_not_on_it() {
    let frame = tiled(&["0:00"]);
    assert_eq!(
        frame.resolution(),
        Resolution::new(WIDTH, HEIGHT + STRIP).expect("a legal raster"),
        "the sheet is the cell and its strip"
    );

    let covered = (0..HEIGHT)
        .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
        .filter(|&(x, y)| brightness(&frame, x, y) != u8::MAX)
        .count();
    assert_eq!(covered, 0, "no pixel of the picture is under the label");

    let (count, middle) = words(&frame, 0, WIDTH, HEIGHT..HEIGHT + STRIP);
    assert!(
        count > 20,
        "the moment is written in the strip, found {count}"
    );
    assert!(
        (middle - f64::from(WIDTH) / 2.0).abs() <= 2.0,
        "a centred moment sits under the middle of its cell, found {middle}"
    );
    assert!(
        brightness(&frame, 1, HEIGHT + 1) < BRIGHT,
        "the strip is a dark panel, not more picture"
    );
}

/// Each cell is stamped with its own moment, under its own cell: two cells
/// given different text carry different amounts of ink, each centred on the
/// cell it belongs to.
#[test]
fn two_cells_are_stamped_with_their_own_moments() {
    let frame = tiled(&["0:00", "0:00:00"]);
    let rows = HEIGHT..HEIGHT + STRIP;
    let (short, first) = words(&frame, 0, WIDTH, rows.clone());
    let (long, second) = words(&frame, WIDTH, WIDTH, rows);

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

/// The issue's own case: a 2x2 of 1920x1080 cells shows every pixel of each
/// frame, and the second row starts below the first row's label rather than
/// on top of it.
#[test]
fn a_two_by_two_of_full_hd_frames_shows_every_pixel_of_each() {
    let cells = (0..4).map(|n| sized(1920, 1080, &format!("0:0{n} · frame {n}")));
    let frame = sheet::tile(cells.collect(), Font::sans(), false).expect("tiles");
    let strip = frame.resolution().height() / 2 - 1080;
    assert_eq!(frame.resolution().width(), 3840);
    assert!(strip >= 100, "the label has room of its own, found {strip}");

    for (left, top) in [(0, 0), (1920, 0), (0, 1080 + strip), (1920, 1080 + strip)] {
        for (x, y) in [
            (left, top),
            (left + 1919, top + 1079),
            (left + 960, top + 1079),
        ] {
            assert_eq!(brightness(&frame, x, y), u8::MAX, "({x}, {y}) is picture");
        }
    }
}

/// A tall, narrow cell gets a label that fits across it, however long — a
/// picture's file name is longer than any timecode: two labels that differ only
/// in their last character come out different, which a label cut short to fit
/// (`0:00 · frame…`, #919) never does, and the ink keeps clear of both edges.
#[test]
fn a_vertical_cells_label_is_written_to_its_last_character() {
    let (width, height) = (108, 192);
    let strip = |label: &str| {
        let frame =
            sheet::tile(vec![sized(width, height, label)], Font::sans(), false).expect("tiles");
        let rows = height..frame.resolution().height();
        let inked: Vec<u32> = (0..width)
            .filter(|&x| rows.clone().any(|y| brightness(&frame, x, y) > BRIGHT))
            .collect();
        (frame.bytes().to_vec(), inked[0], inked[inked.len() - 1])
    };
    let (png, first, last) = strip("2. the-pier-at-dusk-take-4.png");
    let (jpg, ..) = strip("2. the-pier-at-dusk-take-4.jpg");
    assert_ne!(png, jpg, "the end of the label is drawn");
    assert!(
        first >= 2 && last <= width - 3,
        "the label is inside the cell with a margin, inked from {first} to {last}"
    );
}
