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
        dashed: style.strokeDasharray !== "none",
        cap: style.strokeLinecap,
        join: style.strokeLinejoin,
        head: /#([^")]+)"?\)$/.exec(style.markerEnd)?.[1] ?? null,
      });
    }
    return measured.get(mark);
  };
  // A mark inside a clip, a mask or `<defs>` is never drawn itself: it shapes
  // what is, so it is left alone rather than timed and hidden until its turn.
  // Nor is a head `draw` put beside a mark (below): it rides that mark.
  const UNDRAWN = "defs, clipPath, mask, marker, pattern, symbol, [data-kit-head]";
  const marks = (what) => (what instanceof Element
    ? (what.matches(MARKS) ? [what] : [...what.querySelectorAll(MARKS)])
      .filter((mark) => !mark.parentElement?.closest(UNDRAWN))
    : [...what].flatMap(marks));

  // Where a mark is `distance` along itself, and which way it is heading
  // there, in degrees: read off the page's own geometry.
  const pointAt = (mark, distance) => {
    const { length } = measure(mark);
    const at = clamp(distance, 0, length);
    const [a, b] = [Math.max(0, at - 0.5), Math.min(length, at + 0.5)];
    const [here, back, ahead] = [at, a, b].map((d) => mark.getPointAtLength(d));
    return { x: here.x, y: here.y, angle: (Math.atan2(ahead.y - back.y, ahead.x - back.x) * 180) / Math.PI };
  };

  // Puts `element`, an SVG element drawn around its own origin and facing
  // right, on `path` (beside it, in the same user space) at `p` of the way
  // along, by distance, turned to face along it if `orient`.
  const along = (element, path, p, { orient = false } = {}) => {
    const { x, y, angle } = pointAt(path, p * measure(path).length);
    element.setAttribute("transform", `translate(${x} ${y})${orient ? ` rotate(${angle})` : ""}`);
  };

  // What `draw` and `erase` lay over a mark, made once, on its first frame.
  // The pen is what carries the moving dash: the mark itself, or, when its
  // author dashed it, a plain copy of it in a mask over it, so the author's
  // dashes are laid down as the line draws rather than replaced. The head is
  // the mark's `marker-end` drawn as an ordinary group beside it, so it can
  // ride the end being drawn; the marker itself is then turned off. A head
  // coloured `context-stroke` loses that context: colour it on the marker.
  const layers = new WeakMap();
  let masks = 0;
  const dress = (mark, m) => {
    if (layers.has(mark)) return layers.get(mark);
    const layer = { pen: mark, head: null };
    const owner = mark.ownerSVGElement;
    if (m.dashed && owner) {
      const id = `kit-pen-${++masks}`;
      const defs = svg("defs", {}, owner);
      const mask = svg("mask", { id, maskUnits: "userSpaceOnUse", x: -1e5, y: -1e5,
        width: 2e5, height: 2e5 }, defs);
      layer.pen = mark.cloneNode(false);
      for (const name of ["id", "class", "style", "transform", "marker-start", "marker-mid", "marker-end", "mask"]) {
        layer.pen.removeAttribute(name);
      }
      Object.assign(layer.pen.style, { stroke: "white", strokeWidth: m.width, strokeLinecap: m.cap,
        strokeLinejoin: m.join, fill: m.closed ? "white" : "none", fillOpacity: 0 });
      mask.appendChild(layer.pen);
      mark.setAttribute("mask", `url(#${id})`);
    }
    const marker = m.head && document.getElementById(m.head);
    if (marker) {
      const [vb, w, h] = [marker.viewBox.baseVal, marker.markerWidth.baseVal.value, marker.markerHeight.baseVal.value];
      const fit = vb && vb.width ? Math.min(w / vb.width, h / vb.height) : 1;
      const scale = fit * (marker.getAttribute("markerUnits") === "userSpaceOnUse" ? 1 : m.width);
      const angle = marker.getAttribute("orient");
      layer.head = {
        turn: /^auto/.test(angle ?? "") ? null : parseFloat(angle) || 0,
        inner: `scale(${scale}) translate(${-marker.refX.baseVal.value} ${-marker.refY.baseVal.value})`,
        group: svg("g", { transform: mark.getAttribute("transform") ?? "", "data-kit-head": "" }),
      };
      mark.after(layer.head.group);
      const placed = svg("g", {}, layer.head.group);
      placed.append(...[...marker.childNodes].map((child) => child.cloneNode(true)));
      mark.style.markerEnd = "none";
    }
    layers.set(mark, layer);
    return layer;
  };

  // Strokes of `what` swept in document order from `at`, each over its own
  // length at `rate`: drawn on from its start, or, `erasing`, taken away from
  // its start, the line passing through. A stroke is dashed
  // `len (2·len + w)`, so its round cap never shows as a dot past either end.
  const sweep = (what, t, at, { speed = 1500, within = 0, fill = 0.4 }, erasing) => {
    const all = marks(what).map((mark) => [mark, measure(mark)]);
    const total = all.reduce((sum, [, m]) => sum + m.length, 0);
    const rate = within > 0 ? total / within : speed;
    let start = at;
    let done = at;
    for (const [mark, m] of all) {
      const { pen, head } = dress(mark, m);
      // Drawing, a closed shape's fill fades in once its outline is done;
      // erasing, it fades out before the outline goes.
      const lead = erasing && m.closed ? fill : 0;
      const end = start + lead + m.length / rate;
      const p = remap(t, start + lead, end);
      const shown = (erasing ? p < 1 : p > 0) ? "" : "hidden";
      pen.style.strokeDasharray = `${m.length} ${2 * m.length + m.width}`;
      pen.style.strokeDashoffset = (m.length + m.width) * (erasing ? -p : 1 - p);
      mark.style.visibility = shown;
      if (m.closed) {
        const grown = erasing ? 1 - remap(t, start, start + fill) : remap(t, end, end + fill);
        if (pen === mark) mark.style.fillOpacity = m.fill * grown;
        else pen.style.fillOpacity = grown;
      }
      if (head) {
        const tip = pointAt(mark, (erasing ? 1 : p) * m.length);
        head.group.firstChild.setAttribute("transform", `translate(${tip.x} ${tip.y}) rotate(${
          head.turn ?? tip.angle}) ${head.inner}`);
        head.group.style.visibility = shown;
      }
      done = Math.max(done, end + (m.closed && !erasing ? fill : 0));
      start = end;
    }
    return done;
  };

  // Draws the strokes of `what` (an SVG element, its marks, or a list of
  // either) on in document order from `at`, at `speed` user units a second —
  // a long line takes longer than a short one — or all of them `within` that
  // many seconds. A closed shape's fill fades in over `fill` seconds once its
  // outline is done. Answers when the last mark is finished, so the next can
  // start after it. An author's dashes and `marker-end` head are kept.
  const draw = (what, t, at = 0, options = {}) => sweep(what, t, at, options, false);

  // `draw` backwards in time order but not in space: each stroke is taken
  // away from its start, the way it was drawn, so a line passes through.
  // A closed shape's fill fades out over `fill` seconds first.
  const erase = (what, t, at = 0, options = {}) => sweep(what, t, at, options, true);

