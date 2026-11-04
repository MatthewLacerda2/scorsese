//! Marks: the moments a defect was found at, pointed at above the wave.

use scorsese_compositor::waveform::{COLUMNS, Column, Mark};

use super::{BACKGROUND, TOP, marked, pixel};

const MARK: (u8, u8, u8, u8) = (0xf0, 0xb4, 0x3c, 255);

/// A mark sits in the six rows between the caption and the wave's top rule,
/// over the column whose slice holds its moment: one second of four is
/// column 240 of 960.
#[test]
fn an_instant_is_one_column_above_the_wave_at_its_moment() {
    let frame = marked(
        &full(),
        &[Mark {
            from_seconds: 1.0,
            to_seconds: 1.0,
        }],
    );
    let x = 240;
    assert_eq!(pixel(&frame, x, TOP - 7), BACKGROUND);
    assert_eq!(pixel(&frame, x, TOP - 6), MARK);
    assert_eq!(pixel(&frame, x, TOP - 1), MARK);
    assert_ne!(
        pixel(&frame, x, TOP),
        MARK,
        "the wave's own rule is left alone"
    );
    assert_eq!(pixel(&frame, x - 1, TOP - 1), BACKGROUND);
    assert_eq!(pixel(&frame, x + 1, TOP - 1), BACKGROUND);
}

/// A stretch covers its columns, and one past the end of the file is held
/// to the last column drawn rather than drawn past it.
#[test]
fn a_stretch_covers_its_columns_and_stays_on_the_wave() {
    let frame = marked(
        &full(),
        &[Mark {
            from_seconds: 3.0,
            to_seconds: 9.0,
        }],
    );
    assert_eq!(pixel(&frame, 719, TOP - 1), BACKGROUND);
    assert_eq!(pixel(&frame, 720, TOP - 1), MARK);
    assert_eq!(pixel(&frame, COLUMNS as u32 - 1, TOP - 1), MARK);
}

/// A short file draws fewer columns than the width, and its marks land on
/// the columns it drew: the end of a 500-column file is column 499, not the
/// picture's last.
#[test]
fn a_mark_lands_on_the_wave_when_the_wave_is_narrower_than_the_picture() {
    let frame = marked(
        &vec![Column::default(); 500],
        &[Mark {
            from_seconds: 4.0,
            to_seconds: 4.0,
        }],
    );
    assert_eq!(pixel(&frame, 499, TOP - 1), MARK);
    assert_eq!(pixel(&frame, 959, TOP - 1), BACKGROUND);
}

fn full() -> Vec<Column> {
    vec![Column::default(); COLUMNS]
}
