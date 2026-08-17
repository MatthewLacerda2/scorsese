//! Commands out of the blob, a path onto the raster.
//!
//! The same road `shape::closed` takes: build one `tiny_skia::Path`, hand it to
//! [`crate::paint`], and let the layer be composited like any other. An icon is
//! two of those at most — the contours that are stroked, and the handful that
//! are filled — and both are drawn in the one colour the symbol names.
//!
//! Where the square lands is decided here, from the anchor, for the reason
//! `shape::closed` decides it: an icon layer is the size of the whole raster,
//! and a raster-sized layer rests at the origin whatever its anchor says.

use tiny_skia::{Path, PathBuilder};

use scorsese_core::{AnchorX, AnchorY};

use crate::frame::{Frame, Resolution};
use crate::paint;

use super::{Symbol, VIEWBOX};

/// Draws a symbol onto a frame.
///
/// **Stroke first, then the dots**, which is the order the icon is written in
/// and the order that keeps a dot solid: a filled full stop is smaller than the
/// line beside it, and stroking over one would eat into it.
pub(super) fn draw(frame: &mut Frame, symbol: &Symbol) {
    let size = symbol.size;
    if !size.is_finite() || size <= 0.0 {
        return;
    }
    let scale = size / VIEWBOX;
    let origin = origin(size, symbol, frame.resolution());

    if let Some(path) = path(symbol.icon.stroked, scale, origin) {
        paint::stroke_round(frame, &path, symbol.color, symbol.width);
    }
    if let Some(path) = path(symbol.icon.filled, scale, origin) {
        paint::fill(frame, &path, symbol.color);
    }
}

/// One contour list as a path, scaled out of the 24-unit space and placed.
///
/// `None` for an empty list — most icons fill nothing — and for bytes that do
/// not decode, which is a blob that failed to parse rather than a drawing
/// decision.
pub(super) fn path(commands: &[u8], scale: f32, origin: (f32, f32)) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let at = |point: (f32, f32)| (origin.0 + point.0 * scale, origin.1 + point.1 * scale);
    for step in steps(commands)? {
        match step {
            Step::Move(to) => {
                let (x, y) = at(to);
                builder.move_to(x, y);
            }
            Step::Line(to) => {
                let (x, y) = at(to);
                builder.line_to(x, y);
            }
            Step::Cubic(first, second, to) => {
                let (first, second, to) = (at(first), at(second), at(to));
                builder.cubic_to(first.0, first.1, second.0, second.1, to.0, to.1);
            }
            Step::Quad(control, to) => {
                let (control, to) = (at(control), at(to));
                builder.quad_to(control.0, control.1, to.0, to.1);
            }
            Step::Close => builder.close(),
        }
    }
    builder.finish()
}

/// How many commands a contour list holds, or `None` if it does not decode.
///
/// Published up to [`super::Icon::commands`] because "does this icon draw
/// anything at all?" is a question the suite asks of all seventeen hundred of
/// them, and a raster alone cannot tell an empty path from a path drawn off the
/// edge of one.
pub(super) fn count(commands: &[u8]) -> Option<usize> {
    Some(steps(commands)?.len())
}

/// Where the icon's square sits on the raster, in pixels.
fn origin(size: f32, symbol: &Symbol, resolution: Resolution) -> (f32, f32) {
    let (width, height) = (resolution.width() as f32, resolution.height() as f32);
    let x = match symbol.anchor.x {
        AnchorX::Left => 0.0,
        AnchorX::Center => (width - size) / 2.0,
        AnchorX::Right => width - size,
    };
    let y = match symbol.anchor.y {
        AnchorY::Top => 0.0,
        AnchorY::Center => (height - size) / 2.0,
        AnchorY::Bottom => height - size,
    };
    (x, y)
}

/// One step of a contour, in the 24-unit space.
enum Step {
    Move((f32, f32)),
    Line((f32, f32)),
    Cubic((f32, f32), (f32, f32), (f32, f32)),
    Quad((f32, f32), (f32, f32)),
    Close,
}

/// Decodes a whole contour list, or `None` if any byte of it does not read.
///
/// All or nothing on purpose: a list decoded up to the byte that went wrong is
/// an icon missing its last stroke, which looks like a drawing somebody meant.
fn steps(commands: &[u8]) -> Option<Vec<Step>> {
    let mut reader = Reader {
        bytes: commands,
        at: 0,
    };
    let mut steps = Vec::new();
    while reader.at < commands.len() {
        let step = match reader.byte()? {
            1 => Step::Move(reader.point()?),
            2 => Step::Line(reader.point()?),
            3 => Step::Cubic(reader.point()?, reader.point()?, reader.point()?),
            4 => Step::Quad(reader.point()?, reader.point()?),
            5 => Step::Close,
            _ => return None,
        };
        steps.push(step);
    }
    Some(steps)
}

/// A bounds-checked cursor over one contour list.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    fn point(&mut self) -> Option<(f32, f32)> {
        Some((self.coordinate()?, self.coordinate()?))
    }

    fn coordinate(&mut self) -> Option<f32> {
        let bytes: [u8; 4] = self.bytes.get(self.at..self.at + 4)?.try_into().ok()?;
        self.at += 4;
        Some(f32::from_le_bytes(bytes))
    }
}
