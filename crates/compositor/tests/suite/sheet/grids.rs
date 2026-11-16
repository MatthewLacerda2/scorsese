//! How many cells go across and down, past the five `look` walks with
//! (#1563): a glance at a long video fills a sheet of twelve.

use scorsese_compositor::sheet;
use scorsese_compositor::text::Font;

use super::{HEIGHT, STRIP, WIDTH, sized, tiled};

/// Twelve cells, the most a sheet holds, tile four across and three down —
/// what a glance at a long video lays out (#1563) — and a thirteenth is
/// refused rather than dropped.
#[test]
fn twelve_cells_tile_four_by_three_and_a_thirteenth_is_refused() {
    let labels: Vec<String> = (0..sheet::MAX_CELLS).map(|n| format!("0:{n:02}")).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let frame = tiled(&labels);
    assert_eq!(sheet::MAX_CELLS, 12);
    assert_eq!(frame.resolution().width(), 4 * WIDTH);
    assert_eq!(frame.resolution().height(), 3 * (HEIGHT + STRIP));

    let thirteen = (0..=sheet::MAX_CELLS)
        .map(|n| sized(WIDTH, HEIGHT, &n.to_string()))
        .collect();
    assert_eq!(
        sheet::tile(thirteen, Font::sans(), false),
        Err(sheet::SheetError::TooMany { found: 13 })
    );
}

/// Up to nine the grid stays at most three across: two and three are one
/// row, six is three by two, nine three by three.
#[test]
fn up_to_nine_cells_are_at_most_three_across() {
    for (count, columns, rows) in [(2, 2, 1), (3, 3, 1), (6, 3, 2), (9, 3, 3)] {
        let labels: Vec<String> = (0..count).map(|n| format!("0:{n:02}")).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let frame = tiled(&labels);
        assert_eq!(frame.resolution().width(), columns * WIDTH, "{count}");
        assert_eq!(
            frame.resolution().height(),
            rows * (HEIGHT + STRIP),
            "{count}"
        );
    }
}
