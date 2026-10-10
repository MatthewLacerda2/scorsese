//! The traced drawing as an SVG document, in the order it is drawn.
//!
//! Two kinds of mark, each shaped for `kit.draw`, which draws a mark's stroke
//! on along its length and then fades in the fill of a closed one:
//!
//! - **A pen stroke** is a stroke along one edge of an ink line, as wide as the
//!   line is deep on both sides, with no fill, clipped to the ink beside that
//!   edge. Drawing it on uncovers the line as the pen moves.
//! - **A colour** is the shape filled, with a thin stroke of the same colour:
//!   drawing it on sketches its outline, then it fills. The stroke also covers
//!   the hairline that anti-aliasing leaves between two shapes that touch.
//!
//! The clips live in `<defs>` ahead of the drawing, so the drawing itself is
//! one `<g>` holding nothing but the marks, in order.

use std::fmt::Write;

use super::ink::Strand;
use super::outline::{Fit, Mask};
use super::palette::Rgb;
use super::regions::Region;

/// How wide the stroke round a colour is, in the picture's pixels.
const SEAM: f64 = 1.5;

/// Pixels of stroke beyond a strand's depth: half a pixel from a pixel's
/// centre to its edge, and the spline's own wander from the pixels.
const MARGIN: f64 = 2.0;

/// A document being written.
pub(super) struct Drawing {
    /// The clip definitions so far.
    defs: String,
    /// The marks so far.
    marks: String,
    /// What every id begins with.
    name: String,
    /// Clips written.
    clips: usize,
    /// The opening `<svg>` tag.
    head: String,
}

impl Drawing {
    /// An empty drawing the size of the picture.
    pub(super) fn new(width: usize, height: usize, name: &str) -> Self {
        Self {
            head: format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
                 width=\"{width}\" height=\"{height}\">\n"
            ),
            defs: String::new(),
            marks: String::new(),
            name: name.to_owned(),
            clips: 0,
        }
    }

    /// A pen stroke along one edge of an ink line.
    pub(super) fn stroke(&mut self, strand: &Strand, colour: Rgb, fit: Fit) {
        let edge = strand.edge.outline(fit);
        let reach = strand.reach.outline(fit);
        if edge.is_empty() || reach.is_empty() {
            return;
        }
        self.clips += 1;
        let id = format!("{}-ink-{}", self.name, self.clips);
        let _ = writeln!(
            self.defs,
            "<clipPath id=\"{id}\" clipPathUnits=\"userSpaceOnUse\">\
             <path clip-rule=\"evenodd\" d=\"{reach}\"/></clipPath>"
        );
        let width = 2.0 * (f64::from(strand.depth) + MARGIN);
        let _ = writeln!(
            self.marks,
            "<path class=\"ink\" d=\"{edge}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" \
             clip-path=\"url(#{id})\"/>",
            hex(colour)
        );
    }

    /// A shape of one colour.
    pub(super) fn shape(&mut self, region: &Region, width: usize, colour: Rgb, fit: Fit) {
        let mut mask = Mask::around(region.bounds);
        for &pixel in &region.pixels {
            let pixel = pixel as usize;
            let at = mask.at(pixel % width, pixel / width);
            mask.bits[at] = true;
        }
        let d = mask.outline(fit);
        if d.is_empty() {
            return;
        }
        let colour = hex(colour);
        let _ = writeln!(
            self.marks,
            "<path class=\"colour\" d=\"{d}\" fill=\"{colour}\" fill-rule=\"evenodd\" \
             stroke=\"{colour}\" stroke-width=\"{SEAM}\" stroke-linejoin=\"round\"/>"
        );
    }

    /// The finished document.
    pub(super) fn finish(self) -> String {
        let mut svg = self.head;
        if !self.defs.is_empty() {
            let _ = write!(svg, "<defs>\n{}</defs>\n", self.defs);
        }
        let _ = write!(
            svg,
            "<g class=\"drawing\" id=\"{}\">\n{}</g>\n</svg>\n",
            self.name, self.marks
        );
        svg
    }
}

/// A colour as `#rrggbb`.
pub(super) fn hex(colour: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", colour[0], colour[1], colour[2])
}
