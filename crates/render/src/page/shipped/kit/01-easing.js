// scorsese's motion kit (#812), served at https://lib.scorsese/kit.js.
//
// The handful of helpers every timed page was writing for itself: a frame loop
// on the page's own seconds, clamp / lerp / remap, named easings, entrances
// and the clip's exit, stagger, text revealed a piece at a time, a count-up,
// a seeded random, an SVG builder, a constant-speed draw-on and erase that
// keep a line's dashes and arrowhead, a point along a path, text as drawable
// glyph outlines, a camera over a canvas and a person put together from
// Humaaans parts. Loaded with a plain <script src>, it defines one global,
// `kit`.
//
// Every helper is a pure function of the time it is handed: nothing timed is
// kept from one frame to the next (what a helper builds once, on its first —
// `reveal`'s pieces, `draw`'s masks and heads — is the same whenever that
// is), so any frame stands on its own and a still or a piece of a render can
// start anywhere (docs/pages.md, "Seekable, not integrated"). A helper earns its place here by a worked page in
// docs/pages.md using it; anything else waits for pages to need it.
(() => {
  // The page clock reads 100 ms at the page's time zero (clock.js, #606), so
  // seconds counted from 0 would run three frames early at 30 fps and every
  // exit timed back from `duration` would end before the cut (#917).
  const ZERO_MS = 100;

  const clamp = (x, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
  const lerp = (a, b, p) => a + (b - a) * p;

  // The overshoot family and the spring are native keyframes' own curves,
  // ported with their constants: `Easing::apply` in
  // crates/core/src/keyframe/easing.rs is what these must agree with, so a
  // title keyframed natively and a page's lower third land the same way
  // (#1064). The kit's `in`, `out` and `inOut` are cubic and stay so; native's
  // are quadratic and smoothstep.
  const BACK = 1.70158;
  const BACK_BOTH = BACK * 1.525;
  const SPRING_TURNS = 3 * Math.PI;
  const SPRING_DAMPING = 5;
  // `p² · ((c + 1)·p − c)`, in the order that makes both ends exact.
  const wind = (p, pull) => p * p * (pull * (p - 1) + p);

  // The CSS `cubic-bezier()`: x is time and y progress, so find the curve's
  // parameter whose x is `p` (Newton, with bisection behind it for the flat
  // stretches) and read y there. Native's solver, crates/core/src/keyframe/
  // bezier.rs, step for step.
  const bezierAxis = (p1, p2) => {
    const c = 3 * p1;
    const b = 3 * (p2 - p1) - c;
    const a = 1 - c - b;
    return { at: (s) => ((a * s + b) * s + c) * s, slope: (s) => (3 * a * s + 2 * b) * s + c };
  };
  const solve = (axis, x) => {
    let s = x;
    for (let i = 0; i < 8; i++) {
      const error = axis.at(s) - x;
      if (Math.abs(error) < 1e-9 && s >= 0 && s <= 1) return s;
      const slope = axis.slope(s);
      if (Math.abs(slope) < 1e-9) break;
      s -= error / slope;
    }
    let [low, high] = [0, 1];
    s = x;
    for (let i = 0; i < 64; i++) {
      if (axis.at(s) < x) low = s;
      else high = s;
      s = (low + high) / 2;
    }
    return s;
  };
  const cubicBezier = (x1, y1, x2, y2) => {
    if (![x1, x2].every((x) => x >= 0 && x <= 1) || ![y1, y2].every(Number.isFinite)) {
      throw new Error(`kit.ease.cubicBezier(${x1}, ${y1}, ${x2}, ${y2}): x1 and x2 must lie in 0..1, `
        + "or the curve is not a function of time; y1 and y2 may be any number");
    }
    const [x, y] = [bezierAxis(x1, x2), bezierAxis(y1, y2)];
    return (p) => (p <= 0 ? 0 : p >= 1 ? 1 : y.at(solve(x, p)));
  };

  // Each takes progress from 0 to 1 and answers eased progress, 0 at 0 and 1
  // at 1. Entrances ease out, exits ease in. `back`, `backIn`, `backInOut`,
  // `spring` and a `cubicBezier` with a y outside 0..1 go past 0 or 1 on the
  // way: that is the point of them, and `remap` leaves it be.
  const ease = Object.freeze({
    linear: (p) => p,
    in: (p) => p * p * p,
    out: (p) => 1 - (1 - p) ** 3,
    inOut: (p) => (p < 0.5 ? 4 * p * p * p : 1 - (-2 * p + 2) ** 3 / 2),
    // Past 1 and back: a pop that settles. Native's `back_out`.
    back: (p) => 1 - wind(1 - p, BACK),
    // A wind-up: back under 0 first, then off. Native's `back_in`.
    backIn: (p) => wind(p, BACK),
    // Both: winds up, crosses, passes the mark and settles. `back_in_out`.
    backInOut: (p) => (p < 0.5 ? wind(2 * p, BACK_BOTH) / 2 : 1 - wind(2 - 2 * p, BACK_BOTH) / 2),
    // Past the mark by about a sixth, a slight swing back under, then still,
    // landing exactly on 1 at the end. Native's `spring`.
    spring: (p) => 1 - Math.cos(SPRING_TURNS * p) * Math.exp(-SPRING_DAMPING * p) * (1 - p),
    cubicBezier,
  });

  // `x` from [a, b] onto [c, d], held at the ends, eased on the way.
  const remap = (x, a, b, c = 0, d = 1, by = ease.linear) =>
    lerp(c, d, by(clamp(b === a ? (x >= b ? 1 : 0) : (x - a) / (b - a))));

