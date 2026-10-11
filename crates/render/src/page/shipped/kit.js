// scorsese's motion kit (#812), served at https://lib.scorsese/kit.js.
//
// The handful of helpers every timed page was writing for itself: a frame loop
// on the page's own seconds, clamp / lerp / remap, named easings, entrances
// and the clip's exit, stagger, a count-up, a seeded random, an SVG
// builder, a constant-speed draw-on, text as drawable glyph outlines, a
// camera over a canvas and a person put together from Humaaans parts. Loaded
// with a plain <script src>, it defines one global, `kit`.
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
  // from `at`, eased `by`, grouped unless `grouping` is false (a year is not).
  // As the native counter does (crates/core/src/text/counter.rs,
  // crates/compositor/src/text/figure.rs), an easing that overshoots shapes
  // when the count arrives and never what it says, and the figure is padded on
  // the left to the widest one it will show: a figure space (U+2007) where that
  // one has a digit, a punctuation space (U+2008) where it has a separator. Set
  // in tabular figures (`font-variant-numeric: tabular-nums`), a centred line
  // then holds still as it counts. `pad: false` answers the bare figure.
  const count = (t, at, length, to, { from = 0, decimals = 0, locale = "en-US", grouping = true,
    by = ease.out, pad = true } = {}) => {
    const format = (x) => x.toLocaleString(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
      useGrouping: grouping,
      signDisplay: "negative",
    });
    const figure = format(remap(t, at, at + length, from, to, (p) => clamp(by(p))));
    if (!pad) return figure;
    const widest = [format(from), format(to)].reduce((a, b) => ([...b].length > [...a].length ? b : a));
    const room = [...widest].slice(0, Math.max(0, [...widest].length - [...figure].length));
    return room.map((column) => (/[0-9]/.test(column) ? "\u2007" : "\u2008")).join("") + figure;
  };

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

  // What `draw` reads of a mark once, from the page's own geometry: its
  // length, its stroke's width, whether it closes, and the fill opacity its
  // author gave it. Measured, never animated: the frame is still `t`'s alone.
  const MARKS = "path, line, polyline, polygon, rect, circle, ellipse";
  const measured = new WeakMap();
  const measure = (mark) => {
    if (!measured.has(mark)) {
      const style = getComputedStyle(mark);
      const tag = mark.tagName.toLowerCase();
      const length = mark.getTotalLength();
      const [from, to] = [mark.getPointAtLength(0), mark.getPointAtLength(length)];
      measured.set(mark, {
        length,
        width: parseFloat(style.strokeWidth) || 0,
        // A path or polyline closes with a `z`, or by ending where it began.
        closed: ["path", "polyline"].includes(tag)
          ? /z\s*$/i.test(mark.getAttribute("d") || "") || Math.hypot(to.x - from.x, to.y - from.y) < 0.5
          : tag !== "line",
        fill: parseFloat(style.fillOpacity),
      });
    }
    return measured.get(mark);
  };
  // A mark inside a clip, a mask or `<defs>` is never drawn itself: it shapes
  // what is, so it is left alone rather than timed and hidden until its turn.
  const UNDRAWN = "defs, clipPath, mask, marker, pattern, symbol";
  const marks = (what) => (what instanceof Element
    ? (what.matches(MARKS) ? [what] : [...what.querySelectorAll(MARKS)])
      .filter((mark) => !mark.parentElement?.closest(UNDRAWN))
    : [...what].flatMap(marks));

  // Draws the strokes of `what` (an SVG element, its marks, or a list of
  // either) on in document order from `at`, at `speed` user units a second —
  // a long line takes longer than a short one — or all of them `within` that
  // many seconds. A closed shape's fill fades in over `fill` seconds once its
  // outline is done. Answers when the last mark is finished, so the next can
  // start after it. A stroke is dashed `len (2·len + w)` from `len + w`, so
  // its round cap never shows as a dot before it starts.
  const draw = (what, t, at = 0, { speed = 1500, within = 0, fill = 0.4 } = {}) => {
    const all = marks(what).map((mark) => [mark, measure(mark)]);
    const total = all.reduce((sum, [, m]) => sum + m.length, 0);
    const rate = within > 0 ? total / within : speed;
    let start = at;
    let done = at;
    for (const [mark, m] of all) {
      const end = start + m.length / rate;
      const p = remap(t, start, end);
      mark.style.strokeDasharray = `${m.length} ${2 * m.length + m.width}`;
      mark.style.strokeDashoffset = (m.length + m.width) * (1 - p);
      mark.style.visibility = p > 0 ? "" : "hidden";
      if (m.closed) mark.style.fillOpacity = m.fill * remap(t, end, end + fill);
      done = Math.max(done, end + (m.closed ? fill : 0));
      start = end;
    }
    return done;
  };

  // A shipped face as opentype.js reads it (load https://lib.scorsese/
  // opentype.min.js first), by the family name a page's CSS uses or its
  // scorsese name: the nearest weight it ships, or the weight set on a
  // variable one. A promise, so build what uses it in `then`.
  const faces = {};
  const font = (family, { weight = 400, italic = false } = {}) => {
    const key = `${family}/${weight}/${italic}`;
    faces[key] ??= fetch("https://lib.scorsese/fonts/index.json")
      .then((r) => r.json())
      .then((index) => {
        const named = family.toLowerCase();
        const shipped = index.filter((face) => [face.family, face.name]
          .some((name) => name.toLowerCase() === named));
        if (!shipped.length) {
          const families = [...new Set(index.map((face) => face.family))].join(", ");
          throw new Error(`kit.font: no shipped face is called ${family} (there are ${families})`);
        }
        const cuts = shipped.filter((face) => face.italic === italic);
        if (!cuts.length) {
          throw new Error(`kit.font: ${shipped[0].family} ships no ${italic ? "italic" : "upright"} cut`);
        }
        const off = (face) => (face.weight === null ? 0 : Math.abs(face.weight - weight));
        const face = cuts.reduce((best, cut) => (off(cut) < off(best) ? cut : best));
        return fetch(`https://lib.scorsese/fonts/${face.file}`)
          .then((r) => r.arrayBuffer())
          .then((bytes) => {
            const parsed = opentype.parse(bytes);
            const axis = parsed.tables.fvar?.axes.find((a) => a.tag === "wght");
            if (axis) parsed.variation.set({ wght: clamp(weight, axis.minValue, axis.maxValue) });
            return parsed;
          });
      });
    return faces[key];
  };

  // A glyph's commands as path data, each contour closed. Not opentype.js
  // 2.0.0's own `toPathData`, which leaves the `z`s out and, at some
  // positions, writes a coordinate as NaN, cutting the glyph short.
  const outline = (commands) => commands.map(({ type, x1, y1, x2, y2, x, y }, i) => {
    if (type === "Z") return "";
    const at = [x1, y1, x2, y2, x, y].filter((v) => v !== undefined).map((v) => +v.toFixed(2));
    return `${type === "M" && i ? "Z" : ""}${type}${at.join(" ")}`;
  }).join("") + "Z";

  // `text` set in `face` (from `kit.font`) as glyph outlines, each a closed
  // path, inside a new group on `parent`: a `<g data-word>` per word, named
  // as `scorsese.words` names a spoken one (lowercased, bare, `amen@2` the
  // second time), so a word can be drawn on its cue, coloured and popped. Baseline at `y`, lines
  // `leading` × `size` apart, broken at `width` or a "\n", each set from `x`
  // by `align` (start, middle or end). Draw it with `kit.draw`.
  const write = (parent, face, text, { x = 0, y = 0, size = 96, width = Infinity, leading = 1.2,
    align = "start", fill = "currentColor", stroke = fill, strokeWidth = size / 40 } = {}) => {
    const group = svg("g", { fill, stroke, "stroke-width": strokeWidth,
      "stroke-linejoin": "round", "stroke-linecap": "round" }, parent);
    const space = face.getAdvanceWidth(" ", size);
    const lines = [];
    for (const paragraph of text.split("\n")) {
      let line = [];
      let used = 0;
      for (const word of paragraph.split(/\s+/).filter(Boolean)) {
        const advance = face.getAdvanceWidth(word, size);
        if (line.length && used + space + advance > width) {
          lines.push({ line, used });
          line = [];
          used = 0;
        }
        used += (line.length ? space : 0) + advance;
        line.push({ word, advance });
      }
      lines.push({ line, used });
    }
    const said = {};
    lines.forEach(({ line, used }, row) => {
      let left = x - used * { start: 0, middle: 0.5, end: 1 }[align];
      for (const { word, advance } of line) {
        const name = word.toLowerCase().replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
        said[name] = (said[name] ?? 0) + 1;
        const words = svg("g", { "data-word": said[name] > 1 ? `${name}@${said[name]}` : name }, group);
        for (const glyph of face.getPaths(word, left, y + row * size * leading, size)) {
          if (glyph.commands.length) svg("path", { d: outline(glyph.commands) }, words);
        }
        left += advance + space;
      }
    });
    return group;
  };

  // A person put together from Humaaans parts (https://lib.scorsese/humaaans/
  // index.json names them): a `head`, a `body`, and a `bottom` as
  // `standing/<name>` or `sitting/<name>`, in a new group on `parent` in the
  // parts' 300 × 426 frame. `colours` sets regions by name (`skin`, `hair`,
  // `top`, `bottom`…); a region's `-shade` follows it unless given too. A
  // promise of the group, so build what uses it in `then`.
  const PEOPLE = "https://lib.scorsese/humaaans/";
  const person = (parent, { head, body, bottom, colours = {} }) => Promise.all(
    [`head/${head}`, bottom, `body/${body}`].map((part) => fetch(`${PEOPLE}${part}.svg`)
      .then((r) => {
        if (!r.ok) throw new Error(`kit.person: no part ${part} (${PEOPLE}index.json lists them)`);
        return r.text();
      })))
    .then((parts) => {
      const group = svg("g", {}, parent);
      for (const [region, colour] of Object.entries(colours)) {
        group.style.setProperty(`--person-${region}`, colour);
        if (!region.endsWith("-shade") && !(`${region}-shade` in colours)) {
          group.style.setProperty(`--person-${region}-shade`, `color-mix(in oklab, ${colour} 80%, black)`);
        }
      }
      for (const part of parts) {
        const drawn = new DOMParser().parseFromString(part, "image/svg+xml").documentElement;
        group.append(...[...drawn.children].map((mark) => document.importNode(mark, true)));
      }
      return group;
    });

  // Moves a canvas larger than the frame under a fixed camera. `canvas` is
  // drawn at the page's top left; each view is the canvas point `[x, y]` the
  // frame centres on at `zoom`, reached `length` seconds after its `at`,
  // eased in and out. Before the second view, the first holds.
  const camera = (canvas, t, views) => {
    let [x, y, zoom] = [views[0].x, views[0].y, Math.log(views[0].zoom ?? 1)];
    for (const view of views.slice(1)) {
      const p = enter(t, view.at, view.length ?? 1, ease.inOut);
      [x, y, zoom] = [lerp(x, view.x, p), lerp(y, view.y, p), lerp(zoom, Math.log(view.zoom ?? 1), p)];
    }
    const scale = Math.exp(zoom);
    canvas.style.transformOrigin = "0 0";
    canvas.style.transform = `translate(${scorsese.width / 2 - x * scale}px, ${
      scorsese.height / 2 - y * scale}px) scale(${scale})`;
  };

  window.kit = Object.freeze({
    frame, clamp, lerp, remap, ease, enter, exit, rise, stagger, count, random, svg,
    draw, font, write, camera, person,
  });
})();
