//! A set of pixels as SVG path data: the one call into the tracer.
//!
//! visioncortex walks the boundary of a binary image and fits splines to it.
//! Everything around that — which pixels, in what order, drawn how — is this
//! module's caller's; this only turns pixels into `d`.

use std::fmt::Write;

use visioncortex::clusters::Cluster;
use visioncortex::{BinaryImage, CompoundPathElement, PathSimplifyMode, PointF64, PointI32};

/// How closely a boundary is followed: the knobs of visioncortex's spline fit.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fit {
    /// A turn sharper than this, in degrees, stays a corner instead of being
    /// rounded off.
    pub(super) corner: f64,
    /// The length a spline segment is aimed at, in pixels: longer smooths
    /// more away.
    pub(super) segment: f64,
    /// A bend sharper than this, in degrees, starts a new curve.
    pub(super) splice: f64,
}

/// Rounds of smoothing visioncortex gives a boundary. Its own default.
const ITERATIONS: usize = 10;

/// Decimals a coordinate is written with.
const PRECISION: u32 = 1;

/// A clump smaller than this many pixels is not traced: a boundary needs a few
/// pixels to have a shape at all.
const SMALLEST: usize = 3;

/// The pixels of a box of the picture, a `bool` each, row by row.
pub(super) struct Mask {
    /// The box's left edge, in the picture's pixels: one left of the
    /// picture's own edge when the box begins there.
    pub(super) left: isize,
    /// Its top edge, the same way.
    pub(super) top: isize,
    /// Its width.
    pub(super) width: usize,
    /// Its height.
    pub(super) height: usize,
    /// One `true` per pixel that is in.
    pub(super) bits: Vec<bool>,
}

impl Mask {
    /// An empty mask over the box `[left, top, right, bottom)`, with a pixel of
    /// margin all round so that nothing in it touches its edge.
    pub(super) fn around(bounds: [usize; 4]) -> Self {
        let [left, top, right, bottom] = bounds;
        let (width, height) = (right - left + 2, bottom - top + 2);
        Self {
            left: signed(left) - 1,
            top: signed(top) - 1,
            width,
            height,
            bits: vec![false; width * height],
        }
    }

    /// Where the picture's pixel `(x, y)` is in [`Mask::bits`].
    pub(super) fn at(&self, x: usize, y: usize) -> usize {
        let (column, row) = (signed(x) - self.left, signed(y) - self.top);
        usize::try_from(row).unwrap_or(0) * self.width + usize::try_from(column).unwrap_or(0)
    }

    /// Path data for the boundaries of what is in, every one closed: an outline
    /// and, after it, the outline of each hole in it.
    pub(super) fn outline(&self, fit: Fit) -> String {
        let mut image = BinaryImage::new_w_h(self.width, self.height);
        for (at, &bit) in self.bits.iter().enumerate() {
            if bit {
                image.set_pixel(at % self.width, at / self.width, true);
            }
        }
        let mut d = String::new();
        for clump in image.to_clusters(false).iter() {
            if clump.size() < SMALLEST {
                continue;
            }
            let offset = PointI32 {
                x: to_i32(self.left) + clump.rect.left,
                y: to_i32(self.top) + clump.rect.top,
            };
            let traced = Cluster::image_to_compound_path(
                &offset,
                &clump.to_binary_image(),
                PathSimplifyMode::Spline,
                fit.corner.to_radians(),
                fit.segment,
                ITERATIONS,
                fit.splice.to_radians(),
            );
            for element in traced.iter() {
                let _ = write!(d, "{}", data(element));
            }
        }
        d.trim_end().to_owned()
    }
}

/// One closed boundary's path data, in the picture's own coordinates.
fn data(element: &CompoundPathElement) -> String {
    let precision = Some(PRECISION);
    match element {
        CompoundPathElement::PathI32(path) => {
            path.to_svg_string(true, &PointI32::default(), precision)
        }
        CompoundPathElement::PathF64(path) => {
            path.to_svg_string(true, &PointF64::default(), precision)
        }
        CompoundPathElement::Spline(spline) => {
            spline.to_svg_string(true, &PointF64::default(), precision)
        }
    }
}

/// A pixel coordinate as visioncortex holds one. A picture is never two
/// billion pixels across.
fn to_i32(value: isize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A pixel coordinate that can sit one left of, or above, the picture.
fn signed(value: usize) -> isize {
    isize::try_from(value).unwrap_or(isize::MAX)
}
