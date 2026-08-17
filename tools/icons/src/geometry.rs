//! The one shape every element is turned into: a list of path commands in the
//! icon's own 24-unit space.
//!
//! Everything Lucide draws with — `path`, `circle`, `line`, `rect`, `polyline`,
//! `polygon`, `ellipse` — reduces to moves, lines, cubics, quadratics and
//! closes. That reduction happens **here, once, at vendor time**, so the
//! compositor never learns what a `circle` element is: it reads commands and
//! builds a `tiny_skia::Path` out of them.
//!
//! Arcs are the reason this module has trigonometry in it. SVG's `A` is an
//! endpoint parameterisation of an elliptical arc, and no rasteriser draws one
//! directly — tiny-skia has cubics and quadratics and nothing else. Converting
//! it is standard (SVG 1.1, appendix F.6.5) and is done once here rather than
//! on every frame that draws a rounded corner.

use std::f64::consts::PI;

/// A point in the 24-unit space, in the order a command writes it.
pub type Point = (f64, f64);

/// One step of a contour. Absolute coordinates, always — the relative forms of
/// the path grammar are resolved by the parser and never reach the blob.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Start a new contour at this point.
    Move(Point),
    /// A straight segment to this point.
    Line(Point),
    /// A cubic Bézier: two control points, then the end point.
    Cubic(Point, Point, Point),
    /// A quadratic Bézier: one control point, then the end point.
    Quad(Point, Point),
    /// Close the contour back to where it started.
    Close,
}

/// The circle-to-cubic constant, for the four-segment approximation of a full
/// ellipse. The same number `shape::closed` rounds corners with, for the same
/// reason: four of these is a circle to within a fifth of a pixel.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// An axis-aligned ellipse as four cubics, clockwise from its right-hand point.
pub fn ellipse(centre: Point, radii: Point) -> Vec<Command> {
    let (cx, cy) = centre;
    let (rx, ry) = radii;
    let (px, py) = (rx * KAPPA, ry * KAPPA);
    vec![
        Command::Move((cx + rx, cy)),
        Command::Cubic((cx + rx, cy + py), (cx + px, cy + ry), (cx, cy + ry)),
        Command::Cubic((cx - px, cy + ry), (cx - rx, cy + py), (cx - rx, cy)),
        Command::Cubic((cx - rx, cy - py), (cx - px, cy - ry), (cx, cy - ry)),
        Command::Cubic((cx + px, cy - ry), (cx + rx, cy - py), (cx + rx, cy)),
        Command::Close,
    ]
}

/// A rectangle, with quarter-ellipse corners when it has any.
///
/// `radii` is the `rx`/`ry` pair already defaulted for each other — SVG says a
/// rectangle given only one of them is rounded equally on both axes — and is
/// clamped here to half the side it curves along, past which the corners of one
/// side would cross.
pub fn rectangle(origin: Point, size: Point, radii: Point) -> Vec<Command> {
    let (x, y) = origin;
    let (width, height) = size;
    let (right, bottom) = (x + width, y + height);
    let rx = radii.0.clamp(0.0, width / 2.0);
    let ry = radii.1.clamp(0.0, height / 2.0);
    if rx <= 0.0 || ry <= 0.0 {
        return vec![
            Command::Move((x, y)),
            Command::Line((right, y)),
            Command::Line((right, bottom)),
            Command::Line((x, bottom)),
            Command::Close,
        ];
    }
    let (px, py) = (rx * KAPPA, ry * KAPPA);
    vec![
        Command::Move((x + rx, y)),
        Command::Line((right - rx, y)),
        Command::Cubic((right - rx + px, y), (right, y + ry - py), (right, y + ry)),
        Command::Line((right, bottom - ry)),
        Command::Cubic(
            (right, bottom - ry + py),
            (right - rx + px, bottom),
            (right - rx, bottom),
        ),
        Command::Line((x + rx, bottom)),
        Command::Cubic(
            (x + rx - px, bottom),
            (x, bottom - ry + py),
            (x, bottom - ry),
        ),
        Command::Line((x, y + ry)),
        Command::Cubic((x, y + ry - py), (x + rx - px, y), (x + rx, y)),
        Command::Close,
    ]
}

/// How an `A` command's ellipse is described, as the path grammar writes it.
#[derive(Debug, Clone, Copy)]
pub struct Arc {
    /// The radii, before the spec's correction for an ellipse too small to
    /// span the two endpoints.
    pub radii: Point,
    /// How far the ellipse's own x axis is turned from the page's, in degrees.
    pub rotation: f64,
    /// Take the long way round.
    pub large: bool,
    /// Sweep in the direction of increasing angle.
    pub sweep: bool,
}

/// An elliptical arc from `from` to `to`, as cubics — SVG 1.1 F.6.5.
///
/// Degenerate cases are a straight line, which is what the spec says to draw:
/// coincident endpoints produce nothing at all, and a zero radius has no
/// curvature to give.
pub fn arc(from: Point, to: Point, arc: Arc) -> Vec<Command> {
    let (rx, ry) = (arc.radii.0.abs(), arc.radii.1.abs());
    if from == to {
        return Vec::new();
    }
    if rx == 0.0 || ry == 0.0 {
        return vec![Command::Line(to)];
    }
    let phi = arc.rotation.to_radians();
    let (sin, cos) = phi.sin_cos();

    // Into the ellipse's own frame, with the chord's midpoint at the origin.
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;

    // An ellipse too small to reach both endpoints is scaled up until it can,
    // rather than refused. That is the spec's rule and it is the forgiving one.
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    let scale = if lambda > 1.0 { lambda.sqrt() } else { 1.0 };
    let (rx, ry) = (rx * scale, ry * scale);

    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if arc.large == arc.sweep { -1.0 } else { 1.0 };
    let factor = sign * (numerator / denominator).sqrt();
    let cx1 = factor * rx * y1 / ry;
    let cy1 = -factor * ry * x1 / rx;

    let centre = (
        cos * cx1 - sin * cy1 + (from.0 + to.0) / 2.0,
        sin * cx1 + cos * cy1 + (from.1 + to.1) / 2.0,
    );

    let start = angle_of(((x1 - cx1) / rx, (y1 - cy1) / ry));
    let end = angle_of(((-x1 - cx1) / rx, (-y1 - cy1) / ry));
    let mut sweep = end - start;
    if !arc.sweep && sweep > 0.0 {
        sweep -= 2.0 * PI;
    } else if arc.sweep && sweep < 0.0 {
        sweep += 2.0 * PI;
    }

    cubics(centre, (rx, ry), (sin, cos), (start, sweep))
}

/// The angle of a vector from the positive x axis, signed.
fn angle_of(vector: Point) -> f64 {
    vector.1.atan2(vector.0)
}

/// The arc, split into quarter turns and each turn written as one cubic.
///
/// A quarter is where the approximation is good to well under a thousandth of a
/// unit; a half turn in one cubic is visibly not a circle.
fn cubics(centre: Point, radii: Point, rotation: Point, sweep: (f64, f64)) -> Vec<Command> {
    let (start, total) = sweep;
    let segments = (total.abs() / (PI / 2.0)).ceil().max(1.0);
    let step = total / segments;
    // The tangent scaling that makes a cubic match an arc of `step` radians at
    // both ends, in position and in direction.
    let alpha = 4.0 / 3.0 * (step / 4.0).tan();

    let mut commands = Vec::new();
    for segment in 0..segments as u32 {
        let from = start + step * f64::from(segment);
        let to = from + step;
        let (start_point, start_tangent) = on_ellipse(centre, radii, rotation, from);
        let (end_point, end_tangent) = on_ellipse(centre, radii, rotation, to);
        commands.push(Command::Cubic(
            (
                start_point.0 + alpha * start_tangent.0,
                start_point.1 + alpha * start_tangent.1,
            ),
            (
                end_point.0 - alpha * end_tangent.0,
                end_point.1 - alpha * end_tangent.1,
            ),
            end_point,
        ));
    }
    commands
}

/// A point of the rotated ellipse at `theta`, and the tangent there.
fn on_ellipse(centre: Point, radii: Point, rotation: Point, theta: f64) -> (Point, Point) {
    let (sin_phi, cos_phi) = rotation;
    let (rx, ry) = radii;
    let (sin, cos) = theta.sin_cos();
    let point = (
        centre.0 + rx * cos * cos_phi - ry * sin * sin_phi,
        centre.1 + rx * cos * sin_phi + ry * sin * cos_phi,
    );
    let tangent = (
        -rx * sin * cos_phi - ry * cos * sin_phi,
        -rx * sin * sin_phi + ry * cos * cos_phi,
    );
    (point, tangent)
}
