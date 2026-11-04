//! Marks: the moments a defect was found at, pointed at above the wave.

use scorsese_compositor::waveform::{COLUMNS, Mark};

use super::{BACKGROUND, TOP, marked, pixel};

const MARK: (u8, u8, u8, u8) = (0xf0, 0xb4, 0x3c, 255);

/// A mark sits in the six rows between the caption and the wave's top rule,
/// on the same time axis as the stamps: one second of four is a quarter of
/// the way across the 959 columns from the first to the last.
#[test]
fn an_instant_is_one_column_above_the_wave_at_its_moment() {
    let frame = marked(
        &[],
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
/// to the picture rather than drawn off it.
#[test]
fn a_stretch_covers_its_columns_and_stays_on_the_picture() {
    let frame = marked(
        &[],
        &[Mark {
            from_seconds: 3.0,
            to_seconds: 9.0,
        }],
    );
    assert_eq!(pixel(&frame, 718, TOP - 1), BACKGROUND);
    assert_eq!(pixel(&frame, 719, TOP - 1), MARK);
    assert_eq!(pixel(&frame, COLUMNS as u32 - 1, TOP - 1), MARK);
}
