//! The sheet itself, taken of real media.

use scorsese_compositor::Frame;
use scorsese_render::contact::{self, ContactError, Look};

use crate::banded;
use crate::common::ffmpeg::{fixture_dir, generate, tools};

/// The colour at the middle of one cell of the grid, as `(r, g, b)`.
///
/// Read from the cell's middle rather than anywhere near an edge, so the label
/// strip along the bottom is nowhere near what is sampled.
fn cell_colour(image: &Frame, column: u32, row: u32, columns: u32, rows: u32) -> (u8, u8, u8) {
    let cell_width = image.resolution().width() / columns;
    let cell_height = image.resolution().height() / rows;
    let x = column * cell_width + cell_width / 2;
    let y = row * cell_height + cell_height / 2;
    let at = ((y * image.resolution().width() + x) * 4) as usize;
    let bytes = image.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

/// Which of red, green or blue a pixel is nearest, or `None` for none of them.
fn band(colour: (u8, u8, u8)) -> Option<&'static str> {
    let (r, g, b) = colour;
    match (r > 100, g > 100, b > 100) {
        (true, false, false) => Some("red"),
        (false, true, false) => Some("green"),
        (false, false, true) => Some("blue"),
        _ => None,
    }
}

/// The whole point: three frames of a 15-second file land in three different
/// five-second bands, and the sheet shows them in order.
#[test]
fn the_cells_are_the_moments_that_were_asked_for() {
    let dir = fixture_dir("contact-bands");
    let tools = tools();
    let file = banded(&tools, &dir);

    let sheet = contact::sheet(
        &tools,
        &file,
        &Look {
            from_seconds: 2.0,
            to_seconds: None,
            count: 3,
            ..Look::default()
        },
    )
    .expect("a sheet of a 15-second file");

    assert_eq!(sheet.at_seconds, [2.0, 7.0, 12.0]);
    assert_eq!(band(cell_colour(&sheet.image, 0, 0, 3, 1)), Some("red"));
    assert_eq!(band(cell_colour(&sheet.image, 1, 0, 3, 1)), Some("green"));
    assert_eq!(band(cell_colour(&sheet.image, 2, 0, 3, 1)), Some("blue"));
}

/// Five cells are three across and two down, so the sheet is twice as tall as
/// one cell and three times as wide.
#[test]
fn five_frames_tile_three_across_and_two_down() {
    let dir = fixture_dir("contact-grid");
    let tools = tools();
    let file = banded(&tools, &dir);

    let sheet = contact::sheet(&tools, &file, &Look::default()).expect("a sheet");
    assert_eq!(sheet.at_seconds.len(), 3, "a 15-second file holds three");

    let resolution = sheet.image.resolution();
    assert_eq!(resolution.width(), 64 * 3, "three 64px cells across");
    assert_eq!(resolution.height(), 32, "one row");
    assert_eq!(
        sheet.next_from(),
        None,
        "a sheet that reached the end has nowhere to carry on to"
    );
}

/// A file longer than one sheet says where to pick up.
#[test]
fn a_longer_file_says_where_to_carry_on_from() {
    let dir = fixture_dir("contact-long");
    let tools = tools();
    let file = dir.join("long.mp4");
    generate(
        &tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:s=64x32:d=40:r=10",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let sheet = contact::sheet(&tools, &file, &Look::default()).expect("a sheet");
    assert_eq!(sheet.at_seconds, [0.0, 5.0, 10.0, 15.0, 20.0]);
    assert_eq!(sheet.next_from(), Some(25.0));
}

/// An audio file is not broken; there is simply nothing to look at, and saying
/// so is a different answer from failing.
#[test]
fn a_file_with_no_picture_says_so() {
    let dir = fixture_dir("contact-audio");
    let tools = tools();
    let file = dir.join("tone.wav");
    generate(
        &tools,
        &file,
        &["-f", "lavfi", "-i", "sine=frequency=440:duration=2"],
    );

    let refused = contact::sheet(&tools, &file, &Look::default()).expect_err("no video stream");
    assert!(
        matches!(refused, ContactError::NoPicture { .. }),
        "{refused}"
    );
}

/// A source taller than it is wide keeps its shape; nothing is letterboxed,
/// because every cell of one sheet comes from one file at one scale.
#[test]
fn a_vertical_source_keeps_its_shape() {
    let dir = fixture_dir("contact-vertical");
    let tools = tools();
    let file = dir.join("tall.mp4");
    generate(
        &tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=540x960:d=3:r=10",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let sheet = contact::sheet(&tools, &file, &Look::default()).expect("a sheet");
    let resolution = sheet.image.resolution();
    assert_eq!(sheet.at_seconds.len(), 1);
    assert_eq!(resolution.height(), 480, "the longest side is the cap");
    assert_eq!(resolution.width(), 270, "and 9:16 is kept");
}
