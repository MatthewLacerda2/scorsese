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

