//! What was actually drawn, as a measurement rather than a sample.
//!
//! A test that samples a pixel inside a shape says the shape is roughly where it
//! should be. It is nearly blind to one landing in *almost* the right place — a
//! Bézier control point a few pixels out, an origin computed by an addition
//! instead of a division, a stroke half the thickness it was asked for — because
//! the pixel in the middle is inked either way. These numbers are not: the box
//! the ink occupies, and how much ink there is, both move the moment the
//! geometry does.
//!
//! **Coverage is measured in alpha, not in pixels touched.** Every edge here is
//! anti-aliased, so a pixel the drawing half covers carries half the alpha —
//! summing alpha and dividing by 255 gives the area the geometry actually
//! encloses, which is a number that can be worked out on paper and compared
//! against. Counting pixels with *any* ink in them instead would add a fringe
//! the width of the perimeter and answer a different question every time the
//! shape changed size.
//!
//! Shared deliberately: the compositor's strongest assertions are pixels in
//! `crates/golden`, which the mutation harness cannot reach, and its own tests
//! sample points. That is one gap in several modules — shapes, icons, the blur —
//! so the measurement lives in one place for all of them. Each test binary uses
//! a different slice of it, so unused items here are expected rather than dead.
#![allow(dead_code)]

use scorsese_compositor::{BYTES_PER_PIXEL, Frame};

/// The smallest box holding every pixel with any ink in it, in pixels of the
/// raster. Both edges are inclusive, so a single inked pixel has `width() == 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Extent {
    pub(crate) left: u32,
    pub(crate) top: u32,
    pub(crate) right: u32,
    pub(crate) bottom: u32,
}

impl Extent {
    pub(crate) fn width(self) -> u32 {
        self.right - self.left + 1
    }

    pub(crate) fn height(self) -> u32 {
        self.bottom - self.top + 1
    }
}

/// The extent of everything drawn on `frame`, or `None` for a raster nothing
/// touched.
pub(crate) fn extent(frame: &Frame) -> Option<Extent> {
    let (width, height) = dimensions(frame);
    let mut found: Option<Extent> = None;
    for y in 0..height {
        for x in 0..width {
            if alpha(frame, x, y) == 0 {
                continue;
            }
            found = Some(match found {
                None => Extent {
                    left: x,
                    top: y,
                    right: x,
                    bottom: y,
                },
                Some(box_so_far) => Extent {
                    left: box_so_far.left.min(x),
                    top: box_so_far.top.min(y),
                    right: box_so_far.right.max(x),
                    bottom: box_so_far.bottom.max(y),
                },
            });
        }
    }
    found
}

/// How many pixels' worth of ink is on the whole raster.
pub(crate) fn coverage(frame: &Frame) -> f64 {
    let (width, height) = dimensions(frame);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| f64::from(alpha(frame, x, y)))
                .sum::<f64>()
        })
        .sum::<f64>()
        / 255.0
}

/// The same down one column — the measure that reads a shape's thickness at a
/// chosen place, which is how a tapering arrowhead is told from the line it
/// caps.
pub(crate) fn column(frame: &Frame, x: u32) -> f64 {
    let (_, height) = dimensions(frame);
    (0..height)
        .map(|y| f64::from(alpha(frame, x, y)))
        .sum::<f64>()
        / 255.0
}

/// And across one row.
pub(crate) fn row(frame: &Frame, y: u32) -> f64 {
    let (width, _) = dimensions(frame);
    (0..width)
        .map(|x| f64::from(alpha(frame, x, y)))
        .sum::<f64>()
        / 255.0
}

/// Asserts a drawing occupies the box `expected` — `(left, top, right, bottom)`,
/// inclusive — to within a pixel either way.
///
/// One pixel of slack and no more. That is what an anti-aliased edge falling on
/// a pixel boundary costs, and it is far less than any arithmetic mistake this
/// is here to object to.
#[track_caller]
pub(crate) fn assert_extent(frame: &Frame, expected: (u32, u32, u32, u32), what: &str) {
    let found = extent(frame).unwrap_or_else(|| panic!("{what}: nothing was drawn at all"));
    let close = |a: u32, b: u32| a.abs_diff(b) <= 1;
    assert!(
        close(found.left, expected.0)
            && close(found.top, expected.1)
            && close(found.right, expected.2)
            && close(found.bottom, expected.3),
        "{what}: the drawing should occupy {expected:?}, found {found:?}"
    );
}

/// Asserts how much ink is on the raster, to within `slack` pixels' worth.
///
/// `expected` is a number worked out from the geometry rather than read off a
/// run: a 100×100 square is 10000, the ellipse inside it is π/4 of that. Naming
/// the value is the whole point — a measurement asserted to be "in a range"
/// is satisfied by more than one arithmetic.
///
/// `slack` is a pixel for a straight-edged shape and needs to be more for a
/// curved one, because **a cubic is not an arc**: four of them stand *inside* a
/// circle rather than on it, so a curve measures about a tenth of a percent
/// short of its formula — 10 pixels on a circle of radius 50, 12 on an ellipse
/// four times as wide as it is tall. Still a small fraction of what any wrong
/// arithmetic does.
#[track_caller]
pub(crate) fn assert_coverage(frame: &Frame, expected: f64, slack: f64, what: &str) {
    let found = coverage(frame);
    assert!(
        (found - expected).abs() <= slack,
        "{what}: expected {expected} pixels' worth of ink (±{slack}), found {found}"
    );
}

fn dimensions(frame: &Frame) -> (u32, u32) {
    let resolution = frame.resolution();
    (resolution.width(), resolution.height())
}

fn alpha(frame: &Frame, x: u32, y: u32) -> u8 {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL + 3;
    frame.bytes()[at]
}
