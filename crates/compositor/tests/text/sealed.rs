//! A rim goes all the way round. Every contour of a letter is closed before it
//! is stroked, so the rim has no seam where the contour happens to start.
//!
//! The shipped sans starts an `I`'s outline partway along its top edge. Left
//! open there, the stroke ends in two caps instead of turning a corner, and
//! the rim on top of the letter is notched down to the fill.

use scorsese_compositor::text::{self, Edge, Font, Style};
use scorsese_compositor::{BYTES_PER_PIXEL, Frame};
use scorsese_core::Rgba;

use crate::ink::{bounds, canvas, style};

/// Whether the ink across row `y` is one unbroken run.
fn unbroken(frame: &Frame, y: u32) -> bool {
    let width = frame.resolution().width() as usize;
    let row = &frame.bytes()[y as usize * width * BYTES_PER_PIXEL..][..width * BYTES_PER_PIXEL];
    let inked: Vec<bool> = row
        .chunks_exact(BYTES_PER_PIXEL)
        .map(|p| p[3] > 0)
        .collect();
    let first = inked.iter().position(|&ink| ink);
    let last = inked.iter().rposition(|&ink| ink);
    match (first, last) {
        (Some(first), Some(last)) => inked[first..=last].iter().all(|&ink| ink),
        _ => false,
    }
}

#[test]
fn a_rim_has_no_seam_where_its_contour_starts() {
    let mut frame = canvas();
    let style = Style {
        edge: Some(Edge {
            color: Rgba::BLACK,
            width: 6.0,
        }),
        ..style(100.0, Rgba::WHITE)
    };
    text::draw(&mut frame, "I", Font::sans(), &style);
    let (_, top, _, _) = bounds(&frame).expect("ink");
    for y in top..top + 4 {
        assert!(unbroken(&frame, y), "row {y} of the rim is unbroken");
    }
}
