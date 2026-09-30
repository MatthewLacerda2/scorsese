//! Walking an outline by distance: where the point a given fraction of the way
//! along it is, and which way the outline is heading there.
//!
//! **Arc length, not the curve's own parameter.** A cubic's `t = 0.5` is not
//! half way along it — the bow of an S spends its parameter unevenly, fast
//! through the straight middle and slow round the bends — so a line trimmed at
//! `t` would draw on at a speed nobody keyframed, and a dot following it would
//! surge and dawdle. Every fraction here is a fraction of the **distance**
//! along the outline, which is what a person dragging a slider means by "half
//! way".
//!
//! The outline is **flattened** into a polyline first, then measured once. A
//! cubic has no closed-form arc length, and a lookup table of straight pieces is
//! what every renderer uses instead; at the step chosen below the polyline is
//! within a hundredth of a pixel of the curve at any size a frame has, and its
//! length within a few parts in a million.
//!
//! It is measured from **the path that is drawn**, not a second description of
//! the same outline. So where an outline starts, which way round it runs, and
//! how a corner is rounded are whatever [`super::closed`] and [`super::arrow`]
//! draw — a trimmed box and a whole one cannot disagree about where the box is.
//!
//! This is shared ground. A stroke's trim (#583) cuts the drawn line to a span
//! of it, and motion along a path (#584) asks it where a clip is and which way
//! to face — through [`super::measure`] and [`Measured::at`], the same two
//! calls either way.

use tiny_skia::{Path, PathBuilder, PathSegment, Point};

/// The longest straight piece a curve is cut into, in pixels.
///
/// Four pixels keeps a piece's sag off a bend of any radius a diagram has under
/// a tenth of a pixel, which no rasteriser can show.
const STEP: f32 = 4.0;

/// The fewest pieces a curve is cut into, however short: a tiny bend is still
/// a bend, and one straight piece would lose its tangent at each end.
const MIN_PIECES: usize = 4;

/// The most, however long. A curve past `STEP × MAX_PIECES` pixels gets longer
/// pieces rather than an unbounded table.
const MAX_PIECES: usize = 128;

/// An outline flattened into straight pieces, with the distance to each corner
/// of them worked out.
///
/// Built only from an outline with a positive, finite length, so every
/// question it answers has an answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    /// The polyline, from the outline's start to its end — back to the start
    /// again for a closed one. No two consecutive points are the same.
    points: Vec<(f32, f32)>,
    /// How far along the outline each point is, in pixels: `0` for the first,
    /// the whole length for the last.
    along: Vec<f32>,
    /// Whether the outline comes back round to where it began.
    closed: bool,
}

/// A place on an outline and the way it is heading there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// Where, in pixels from the raster's top-left.
    pub position: (f32, f32),
    /// Which way the outline runs through it, as a direction of length one,
    /// pointing from the outline's start toward its end.
    pub tangent: (f32, f32),
}

impl Station {
    /// The tangent as an angle, in **degrees clockwise** from pointing right —
    /// the convention `transform.rotation` uses, so a layer turned by this
    /// faces along the outline.
    pub fn heading(&self) -> f32 {
        self.tangent.1.atan2(self.tangent.0).to_degrees()
    }
}

impl Measured {
    /// Measures the first contour of `path`.
    ///
    /// `None` when it has no length at all — a single point, or numbers that
    /// are not numbers. Every outline this module draws is one contour, so the
    /// first is the whole of it.
    pub(crate) fn of_path(path: &Path) -> Option<Self> {
        let mut points: Vec<(f32, f32)> = Vec::new();
        let mut closed = false;
        for segment in path.segments() {
            match segment {
                PathSegment::MoveTo(point) => {
                    if !points.is_empty() {
                        break;
                    }
                    points.push(xy(point));
                }
                PathSegment::LineTo(point) => push(&mut points, xy(point)),
                PathSegment::QuadTo(control, point) => {
                    let from = *points.last()?;
                    let [c, p] = [xy(control), xy(point)];
                    flatten(&mut points, &[from, c, p], |t| quad(from, c, p, t));
                }
                PathSegment::CubicTo(first, second, point) => {
                    let from = *points.last()?;
                    let [c1, c2, p] = [xy(first), xy(second), xy(point)];
                    flatten(&mut points, &[from, c1, c2, p], |t| {
                        cubic(from, c1, c2, p, t)
                    });
                }
                PathSegment::Close => {
                    if let Some(&first) = points.first() {
                        push(&mut points, first);
                    }
                    closed = true;
                    break;
                }
            }
        }
        let mut along = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (at, point) in points.iter().enumerate() {
            if let Some(before) = at.checked_sub(1).map(|before| points[before]) {
                total += gap(before, *point);
            }
            along.push(total);
        }
        if points.len() < 2 || !total.is_finite() || total <= 0.0 {
            return None;
        }
        Some(Self {
            points,
            along,
            closed,
        })
    }

    /// How long the outline is, in pixels.
    pub fn length(&self) -> f32 {
        self.along.last().copied().unwrap_or_default()
    }

    /// Whether it comes back round to where it began — a box or an ellipse,
    /// rather than an arrow.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// The place `fraction` of the way along, by distance, and the heading
    /// there.
    ///
    /// `fraction` is clamped to `0.0..=1.0`, and one that is not a number is
    /// the start. **At a corner, the heading is the one it arrives with**: the
    /// tip of a line trimmed to end exactly on a corner points the way the line
    /// was going, not the way it is about to turn.
    pub fn at(&self, fraction: f32) -> Station {
        let distance = self.distance(fraction);
        let last = self.points.len() - 2;
        let piece = self
            .along
            .partition_point(|&along| along < distance)
            .saturating_sub(1)
            .min(last);
        let (from, to) = (self.points[piece], self.points[piece + 1]);
        let span = self.along[piece + 1] - self.along[piece];
        let t = ((distance - self.along[piece]) / span).clamp(0.0, 1.0);
        let length = gap(from, to);
        Station {
            position: (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t),
            tangent: ((to.0 - from.0) / length, (to.1 - from.1) / length),
        }
    }

    /// The stretch of outline from `start` to `end`, as fractions of its
    /// length, as an open path to stroke.
    ///
    /// `None` when there is nothing between them — `end` at or before
    /// `start`, after both are clamped to `0.0..=1.0`.
    pub(crate) fn between(&self, start: f32, end: f32) -> Option<Path> {
        let (from, to) = (self.distance(start), self.distance(end));
        if to <= from {
            return None;
        }
        let first = self.at(start).position;
        let mut builder = PathBuilder::new();
        builder.move_to(first.0, first.1);
        for (point, along) in self.points.iter().zip(&self.along) {
            if *along > from && *along < to {
                builder.line_to(point.0, point.1);
            }
        }
        let last = self.at(end).position;
        builder.line_to(last.0, last.1);
        builder.finish()
    }

    /// How far along `fraction` is, in pixels, clamped to the outline.
    pub(crate) fn distance(&self, fraction: f32) -> f32 {
        let fraction = if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        };
        fraction * self.length()
    }
}

/// Appends `point` unless it is where the polyline already is — a piece of no
/// length has no direction, and would make a tangent out of nothing.
fn push(points: &mut Vec<(f32, f32)>, point: (f32, f32)) {
    if points.last() != Some(&point) {
        points.push(point);
    }
}

/// Cuts one curve into straight pieces, as many as its control polygon's
/// length asks for, ending exactly on its last control point.
fn flatten(points: &mut Vec<(f32, f32)>, control: &[(f32, f32)], at: impl Fn(f32) -> (f32, f32)) {
    let reach: f32 = control.windows(2).map(|pair| gap(pair[0], pair[1])).sum();
    let pieces = if reach.is_finite() {
        ((reach / STEP).ceil() as usize).clamp(MIN_PIECES, MAX_PIECES)
    } else {
        MIN_PIECES
    };
    for piece in 1..pieces {
        push(points, at(piece as f32 / pieces as f32));
    }
    if let Some(&end) = control.last() {
        push(points, end);
    }
}

fn quad(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c) = (u * u, 2.0 * u * t, t * t);
    (
        a * p0.0 + b * p1.0 + c * p2.0,
        a * p0.1 + b * p1.1 + c * p2.1,
    )
}

fn cubic(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
        a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
    )
}

fn xy(point: Point) -> (f32, f32) {
    (point.x, point.y)
}

/// How far apart two points are, in pixels.
fn gap((ax, ay): (f32, f32), (bx, by): (f32, f32)) -> f32 {
    (bx - ax).hypot(by - ay)
}
