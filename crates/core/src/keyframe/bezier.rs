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
        let mut s = x;
        for _ in 0..NEWTON_STEPS {
            let error = self.at(s) - x;
            // Only a root on the curve counts: a cubic has others off either
            // end of it, and Newton can wander onto one.
            if error.abs() < EPSILON && (0.0..=1.0).contains(&s) {
                return s;
            }
            let slope = self.slope(s);
            if slope.abs() < EPSILON {
                break;
            }
            s -= error / slope;
        }
        let (mut low, mut high) = (0.0, 1.0);
        s = x;
        for _ in 0..BISECTION_STEPS {
            let at = self.at(s);
            if (at - x).abs() < EPSILON {
                break;
            }
            if at < x {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
        s
    }
}
