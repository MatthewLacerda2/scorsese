//! The CSS `cubic-bezier()` timing function, solved the way browsers solve it.
//!
//! The curve runs from `(0, 0)` to `(1, 1)` through two control points, and
//! its **x is time and its y is progress** — so easing a moment means finding
//! the curve parameter whose x is that moment, then reading y there. There is
//! no closed form worth writing for the first half, so it is Newton's method
//! from a good first guess, with bisection behind it for the flat stretches
//! where Newton's step is a division by almost nothing. That is WebKit's
//! `UnitBezier`, and matching it is the point: `[0.25, 0.1, 0.25, 1]` should
//! move exactly as `ease` does in a browser, because that is where anyone who
//! types four numbers got them from.

/// How close to the asked-for moment the solver gets, in progress through the
/// segment. A frame is a thousandth of a segment only when the segment is a
/// thousand frames long, so this is several orders of magnitude finer than any
/// frame anyone renders.
const EPSILON: f64 = 1e-9;

/// Newton converges in two or three steps on any curve that is a function of
/// time; this many is the budget before handing over to bisection.
const NEWTON_STEPS: usize = 8;

/// Each halves the interval, so this is past `f64`'s precision on `0..=1` —
/// a bound on the loop rather than a precision anyone waits for.
const BISECTION_STEPS: usize = 64;

/// Eased progress at `progress`, for control points `[x1, y1, x2, y2]`.
///
/// The ends are answered outright rather than solved: a keyframe's own value
/// has to read back exactly, and `a + b + c` summed in floating point is only
/// nearly one. `y` is not clamped anywhere — a `y` outside `0..=1` is how a
/// curve overshoots, and overshooting is what someone writing one asked for.
pub(super) fn ease([x1, y1, x2, y2]: [f64; 4], progress: f64) -> f64 {
    if progress <= 0.0 {
        return 0.0;
    }
    if progress >= 1.0 {
        return 1.0;
    }
    Cubic::through(y1, y2).at(Cubic::through(x1, x2).solve(progress))
}

/// One axis of the curve, as a polynomial in the curve parameter `s`.
///
/// Expanded from the Bernstein form once, so evaluating it is three
/// multiply-adds — which matters because it runs per sample on a volume ramp.
struct Cubic {
    a: f64,
    b: f64,
    c: f64,
}

impl Cubic {
    /// The axis that starts at `0`, ends at `1`, and is pulled by `p1` and `p2`.
    fn through(p1: f64, p2: f64) -> Self {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        Self {
            a: 1.0 - c - b,
            b,
            c,
        }
    }

    fn at(&self, s: f64) -> f64 {
        ((self.a * s + self.b) * s + self.c) * s
    }

    fn slope(&self, s: f64) -> f64 {
        (3.0 * self.a * s + 2.0 * self.b) * s + self.c
    }

    /// The parameter at which this axis reads `x`.
    ///
    /// Bounded however badly shaped the curve is: validation refuses control
    /// points that make x run backwards, but the evaluator is not entitled to
    /// assume a document was validated, and a loop that never ends is a render
    /// that never ends.
    fn solve(&self, x: f64) -> f64 {
        self.newton(x).unwrap_or_else(|| self.bisect(x))
    }

    /// Newton's answer, or `None` when it did not land on the curve within its
    /// budget — a flat stretch, or a wander off either end.
    ///
    /// A separate step so its tests can see it: bisection lands on the same
    /// answer whenever Newton fails, so through [`Cubic::solve`] a Newton that
    /// never converged would read exactly like one that did, only slower.
    fn newton(&self, x: f64) -> Option<f64> {
        let mut s = x;
        for _ in 0..NEWTON_STEPS {
            let error = self.at(s) - x;
            // Only a root on the curve counts: a cubic has others off either
            // end of it, and Newton can wander onto one.
            if error.abs() < EPSILON && (0.0..=1.0).contains(&s) {
                return Some(s);
            }
            let slope = self.slope(s);
            if slope.abs() < EPSILON {
                return None;
            }
            s -= error / slope;
        }
        None
    }

    /// The slow, certain fallback: halve the curve's parameter range
    /// [`BISECTION_STEPS`] times.
    ///
    /// It runs its whole budget rather than stopping once within
    /// [`EPSILON`]. The early stop saved a few dozen multiply-adds on the rare
    /// sample Newton cannot answer, and nothing could observe it — its
    /// mutations survived every test there is (#609) — so it was a branch
    /// nobody could hold to anything.
    fn bisect(&self, x: f64) -> f64 {
        let (mut low, mut high) = (0.0, 1.0);
        let mut s = x;
        for _ in 0..BISECTION_STEPS {
            if self.at(s) < x {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The x axes of CSS's named curves, of a common overshoot, and of a
    /// straight line: every shape a person copies out of a stylesheet.
    const AXES: [(f64, f64); 6] = [
        (0.25, 0.25),
        (0.42, 1.0),
        (0.0, 0.58),
        (0.42, 0.58),
        (0.34, 0.64),
        (0.2, 0.8),
    ];

    fn moments() -> impl Iterator<Item = f64> {
        (1..100).map(|i| f64::from(i) / 100.0)
    }

    /// Checked against the axis itself rather than a formula written out a
    /// second time: a central difference agrees with the true slope to
    /// about `h²`, far inside the tolerance and far outside any slip.
    #[test]
    fn the_slope_is_the_axis_derivative() {
        let h = 1e-6;
        for (p1, p2) in [(0.25, 0.25), (0.9, -0.7), (0.1, 1.8)] {
            let axis = Cubic::through(p1, p2);
            for s in [0.0, 0.2, 0.5, 0.77, 1.0] {
                let numeric = (axis.at(s + h) - axis.at(s - h)) / (2.0 * h);
                let slope = axis.slope(s);
                assert!((slope - numeric).abs() < 1e-6, "{p1},{p2} at {s}");
            }
        }
    }

    /// Newton, on its own, answers every moment of a well-shaped curve. Without
    /// this the solver could silently always bisect: the answer would be the
    /// same and every sample would cost eight times as much.
    #[test]
    fn newton_lands_on_every_well_shaped_curve() {
        for (p1, p2) in AXES {
            let axis = Cubic::through(p1, p2);
            for x in moments() {
                let s = axis.newton(x).expect("Newton lands");
                assert!((0.0..=1.0).contains(&s), "{p1},{p2} at {x}: {s}");
                assert!((axis.at(s) - x).abs() < EPSILON, "{p1},{p2} at {x}");
            }
        }
    }

    #[test]
    fn bisection_finds_what_newton_finds() {
        for (p1, p2) in AXES {
            let axis = Cubic::through(p1, p2);
            for x in moments() {
                let s = axis.bisect(x);
                assert!((axis.at(s) - x).abs() < EPSILON, "{p1},{p2} at {x}");
                let newton = axis.newton(x).expect("Newton lands");
                assert!((s - newton).abs() < 1e-8, "{p1},{p2} at {x}");
            }
        }
    }

    /// `(1, 0)` stands still at its middle: x is `0.5 + 4(s - 0.5)³`, whose
    /// slope there is zero. Just off it, Newton's step would divide by almost
    /// nothing, so it hands over — and bisection still answers.
    #[test]
    fn a_flat_stretch_is_left_to_bisection() {
        let flat = Cubic::through(1.0, 0.0);
        let x = 0.5 + 5e-6;
        assert_eq!(flat.newton(x), None);
        let s = flat.solve(x);
        assert!((flat.at(s) - x).abs() < EPSILON, "{s}");
    }
}
