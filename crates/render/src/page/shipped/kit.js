// scorsese's motion kit (#812), served at https://lib.scorsese/kit.js.
//
// The handful of helpers every timed page was writing for itself: a frame loop
// on the page's own seconds, clamp / lerp / remap, named easings, entrances
// and the clip's exit, stagger, a count-up, a seeded random and an SVG
// builder. Loaded with a plain <script src>, it defines one global, `kit`.
//
// Every helper is a pure function of the time it is handed: nothing is kept
// from one frame to the next, so any frame stands on its own and a still or a
// piece of a render can start anywhere (docs/pages.md, "Seekable, not
// integrated"). A helper earns its place here by a worked page in
// docs/pages.md using it; anything else waits for pages to need it.
(() => {
  // The page clock reads 100 ms at the page's time zero (clock.js, #606), so
  // seconds counted from 0 would run three frames early at 30 fps and every
  // exit timed back from `duration` would end before the cut (#917).
  const ZERO_MS = 100;

  const clamp = (x, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
  const lerp = (a, b, p) => a + (b - a) * p;

  // Each takes progress from 0 to 1 and answers eased progress, 0 at 0 and 1
  // at 1. Entrances ease out, exits ease in.
  const ease = Object.freeze({
    linear: (p) => p,
    in: (p) => p * p * p,
    out: (p) => 1 - (1 - p) ** 3,
    inOut: (p) => (p < 0.5 ? 4 * p * p * p : 1 - (-2 * p + 2) ** 3 / 2),
    // Past 1 and back: a pop that settles.
    back: (p) => 1 + 2.70158 * (p - 1) ** 3 + 1.70158 * (p - 1) ** 2,
  });

  // `x` from [a, b] onto [c, d], held at the ends, eased on the way.
  const remap = (x, a, b, c = 0, d = 1, by = ease.linear) =>
    lerp(c, d, by(clamp(b === a ? (x >= b ? 1 : 0) : (x - a) / (b - a))));

  // Calls `draw(t)` on every frame with the page's seconds: 0 on its first
  // frame, `scorsese.duration` on the clip's last.
  const frame = (draw) => {
    const tick = (now) => {
      draw((now - ZERO_MS) / 1000);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  };

  // 0 before `at`, 1 once `length` seconds have passed, eased out between.
  const enter = (t, at = 0, length = 0.6, by = ease.out) =>
    remap(t, at, at + length, 0, 1, by);

  // 1 until the clip's last `length` seconds, then down to 0 on its last
  // frame, eased in — however long the clip is cut.
  const exit = (t, length = 0.3, by = ease.in) =>
    remap(t, scorsese.duration - length, scorsese.duration, 1, 0, by);

  // Fades `element` in while it rises `distance` px into place from `at`,
  // and out with the clip's last `out` seconds (0 to stay to the end).
  const rise = (element, t, at = 0, { length = 0.6, distance = 40, out = 0.3 } = {}) => {
    const p = enter(t, at, length);
    element.style.opacity = p * (out > 0 ? exit(t, out) : 1);
    element.style.transform = `translateY(${(1 - p) * distance}px)`;
  };

  // When the `index`th of a row enters: `step` seconds after the one before.
  const stagger = (index, step = 0.15, first = 0) => first + index * step;

  // The figure a count-up shows at `t`: `from` to `to` over `length` seconds
  // from `at`, eased out, with grouping. Show it in tabular figures
  // (`font-variant-numeric: tabular-nums`) so it keeps its width.
  const count = (t, at, length, to, { from = 0, decimals = 0, locale = "en-US" } = {}) =>
    remap(t, at, at + length, from, to, ease.out).toLocaleString(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });

  // A random number generator that gives the same sequence for the same seed
  // on every frame and every machine (mulberry32): call it once per number,
  // in the same order each frame, or draw them all once up front.
  const random = (seed) => {
    let s = seed >>> 0;
    return () => {
      s = (s + 0x6d2b79f5) >>> 0;
      let r = Math.imul(s ^ (s >>> 15), 1 | s);
      r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
      return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
    };
  };

  // An SVG element with its attributes set, appended to `parent` if given.
  const svg = (tag, attributes = {}, parent = null) => {
    const element = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
    if (parent) parent.appendChild(element);
    return element;
  };

  window.kit = Object.freeze({
    frame, clamp, lerp, remap, ease, enter, exit, rise, stagger, count, random, svg,
  });
})();
