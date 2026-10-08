// What a page's layout gets wrong that a thumbnail hides (#813).
//
// Evaluated after a sampled frame is drawn, as `(this)(margin)`, and answers
// with findings: text running off the frame, text inside the safe margin, text
// overflowing the box painted behind it, and two blocks of text overlapping.
// Each is `{ key, message }`: the message is what the author reads, the key
// adds the geometry, so the capture can tell a layout held wrong (the same key
// on two samples running) from an entrance passing through.
//
// Text only, and measured by its lines rather than its element: a caption
// centred in a wide box is as wide as its words, not its box. Each line is
// trimmed to its font size, because a line's box carries the font's ascent
// and descent and would make tight but correct type look like a collision.
// Precision over recall throughout: text that is hidden, transparent or too
// faint to read is skipped, and anything positioned out of its box on purpose
// is not checked against that box.
//
// Each line is measured as much of it as can be seen: cut to the boxes that
// clip it (`overflow` other than visible, and an `inset()` clip-path), so a
// word a mask has slid out of view says nothing, and one half out is measured
// by the half in. A clip edge at or past the frame's is left to the frame
// checks, which say more about it. Masks (`mask-image`), SVG clip paths and
// clip-path shapes other than `inset()` are not read: text under one is
// measured whole.
//
// A line a clipping box cuts partway is sliced on screen, so it is a finding
// too (#927), naming the box and the side. Not when the box draws an ellipsis
// (the line's rects may report its unellipsised width) or scrolls on that axis
// (it is meant to hold more than it shows).
(margin) => {
  const W = innerWidth;
  const H = innerHeight;
  const safe = { left: W * margin, right: W * margin, top: H * margin, bottom: H * margin };
  const SLACK = 1;
  const findings = [];
  const px = (n) => `${Math.round(n)} px`;
  const quote = (text) => {
    const flat = text.replace(/\s+/g, " ").trim();
    return `"${flat.length > 40 ? `${flat.slice(0, 39)}…` : flat}"`;
  };
  const named = (el) => {
    const tag = el.tagName.toLowerCase();
    const id = el.id ? `#${el.id}` : "";
    const classes = [...el.classList].slice(0, 2).map((c) => `.${c}`).join("");
    return `<${tag}${id}${classes}>`;
  };
  const alpha = (colour) => {
    if (colour === "transparent") return 0;
    const parts = colour.match(/^rgba?\(([^)]*)\)$/);
    if (!parts) return 1;
    const values = parts[1].split(/[\s,/]+/).filter(Boolean);
    return values.length > 3 ? parseFloat(values[3]) : 1;
  };
  const opacity = (el) => {
    let product = 1;
    for (let at = el; at; at = at.parentElement) product *= parseFloat(getComputedStyle(at).opacity);
    return product;
  };
  const isBox = (el) => !["inline", "contents"].includes(getComputedStyle(el).display);
  const paints = (el) => {
    const style = getComputedStyle(el);
    const bordered = ["Top", "Right", "Bottom", "Left"].every(
      (side) => parseFloat(style[`border${side}Width`]) > 0 && style[`border${side}Style`] !== "none",
    );
    return alpha(style.backgroundColor) > 0 || style.backgroundImage !== "none" ||
      style.boxShadow !== "none" || bordered;
  };
  const placed = (el) => ["absolute", "fixed"].includes(getComputedStyle(el).position);
  // The box painted behind a block of text: the nearest ancestor that paints,
  // unless the text is positioned out of the flow on the way to it, or the box
  // is a full-frame background (the frame checks already cover that).
  const cardOf = (block) => {
    for (let at = block; at && at !== document.body && at !== document.documentElement; at = at.parentElement) {
      if (paints(at)) {
        const box = at.getBoundingClientRect();
        return box.width >= W - SLACK && box.height >= H - SLACK ? null : at;
      }
      if (placed(at)) return null;
    }
    return null;
  };
  const trimmed = (rect, size) => {
    const inset = Math.max(0, (rect.height - size) / 2);
    return { left: rect.left, right: rect.right, top: rect.top + inset, bottom: rect.bottom - inset };
  };
  const meet = (a, b) => ({
    left: Math.max(a.left, b.left), right: Math.min(a.right, b.right),
    top: Math.max(a.top, b.top), bottom: Math.min(a.bottom, b.bottom),
  });
  const EVERYWHERE = { left: -Infinity, right: Infinity, top: -Infinity, bottom: Infinity };
  const CLIPPING = ["hidden", "clip", "scroll", "auto"];
  // The box `inset(t r b l)` leaves of `el`, or everywhere for any other shape.
  const insetOf = (el, clipPath, box) => {
    const inner = clipPath.match(/^inset\(([^)]*)\)/);
    if (!inner) return EVERYWHERE;
    const values = inner[1].split(/\s+round\s+/)[0].trim().split(/\s+/);
    if (values.some((v) => !/^-?[\d.]+(px|%)$/.test(v))) return EVERYWHERE;
    const [t, r = t, b = t, l = r] = values;
    // Lengths are in the element's own pixels; its box may be scaled.
    const scale = el.offsetWidth > 0 ? box.width / el.offsetWidth : 1;
    const length = (v, along) => parseFloat(v) * (v.endsWith("%") ? along / 100 : scale);
    return {
      left: box.left + length(l, box.width), right: box.right - length(r, box.width),
      top: box.top + length(t, box.height), bottom: box.bottom - length(b, box.height),
    };
  };
  // What `el` clips its own content to.
  const ownClip = (el, style) => {
    const box = el.getBoundingClientRect();
    let clip = insetOf(el, style.clipPath, box);
    if (CLIPPING.includes(style.overflowX)) clip = meet(clip, { ...EVERYWHERE, left: box.left, right: box.right });
    if (CLIPPING.includes(style.overflowY)) clip = meet(clip, { ...EVERYWHERE, top: box.top, bottom: box.bottom });
    return clip;
  };
  const transformed = (style) =>
    style.transform !== "none" || style.filter !== "none" || style.perspective !== "none";
  // Where the content of `el` can be seen: its own clip and its clipping
  // ancestors', with the element each side's edge came from. An `overflow`
  // clips only what it contains, so past a box placed absolutely the walk
  // skips ancestors until its containing block (a fixed box's is a transformed
  // ancestor, or the frame). `body` and the root are not read: their overflow
  // is the frame's.
  const SIDES = ["left", "right", "top", "bottom"];
  const tighter = { left: (a, b) => a > b, right: (a, b) => a < b, top: (a, b) => a > b, bottom: (a, b) => a < b };
  const seen = new Map();
  const visibleArea = (el) => {
    if (seen.has(el)) return seen.get(el);
    const area = { ...EVERYWHERE };
    const by = {};
    let escaped = null;
    for (let at = el; at && at !== document.body && at !== document.documentElement; at = at.parentElement) {
      const style = getComputedStyle(at);
      const holds = escaped === null || transformed(style) ||
        (escaped === "absolute" && style.position !== "static");
      if (holds) escaped = null;
      const clip = holds ? ownClip(at, style) : insetOf(at, style.clipPath, at.getBoundingClientRect());
      for (const side of SIDES) {
        if (tighter[side](clip[side], area[side])) [area[side], by[side]] = [clip[side], at];
      }
      if (escaped === null && ["absolute", "fixed"].includes(style.position)) escaped = style.position;
    }
    const atFrame = { left: area.left <= SLACK, right: area.right >= W - SLACK, top: area.top <= SLACK, bottom: area.bottom >= H - SLACK };
    for (const side of SIDES) {
      if (atFrame[side]) [area[side], by[side]] = [EVERYWHERE[side], undefined];
    }
    const found = { area, by };
    seen.set(el, found);
    return found;
  };
  // Whether a cut by `box` on `side` is a mistake rather than how the box works.
  const slices = (box, side) => {
    const style = getComputedStyle(box);
    const overflow = side === "left" || side === "right" ? style.overflowX : style.overflowY;
    return style.textOverflow !== "ellipsis" && !["scroll", "auto"].includes(overflow);
  };
  // The deepest cut a clipping box makes into lines still partly in view, as
  // `{ box, side, depth }`, or null.
  const cutOf = (lines, el) => {
    const { area, by } = visibleArea(el);
    let deepest = null;
    for (const line of lines) {
      const shown = meet(line, area);
      if (shown.right <= shown.left || shown.bottom <= shown.top) continue;
      for (const side of SIDES) {
        const depth = Math.abs(shown[side] - line[side]);
        if (by[side] && depth > SLACK && (!deepest || depth > deepest.depth) && slices(by[side], side)) {
          deepest = { box: by[side], side, depth };
        }
      }
    }
    return deepest;
  };
  const visible = (lines, el) => lines.map((line) => meet(line, visibleArea(el).area))
    .filter((r) => r.right > r.left && r.bottom > r.top);

  // Blocks of text: the text nodes under one box, together.
  const blocks = new Map();
  const walker = document.createTreeWalker(document.body || document.documentElement, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const parent = node.parentElement;
    if (!parent || !node.data.trim() || parent instanceof SVGElement) continue;
    if (["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"].includes(parent.tagName)) continue;
    const style = getComputedStyle(parent);
    if (style.visibility !== "visible" || alpha(style.color) === 0) continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    const size = parseFloat(style.fontSize);
    const measured = [...range.getClientRects()].filter((r) => r.width > 0 && r.height > 0).map((r) => trimmed(r, size));
    const lines = visible(measured, parent);
    if (!lines.length) continue;
    let block = parent;
    while (!isBox(block) && block.parentElement) block = block.parentElement;
    const entry = blocks.get(block) || { text: "", lines: [], opacity: opacity(parent), svg: false, cut: null };
    entry.text += node.data;
    entry.lines.push(...lines);
    const cut = cutOf(measured, parent);
    if (cut && (!entry.cut || cut.depth > entry.cut.depth)) entry.cut = cut;
    entry.opacity = Math.max(entry.opacity, opacity(parent));
    blocks.set(block, entry);
  }
  for (const text of document.querySelectorAll("svg text")) {
    if (text.parentElement && text.parentElement.closest("text")) continue;
    const style = getComputedStyle(text);
    const box = text.getBoundingClientRect();
    if (!text.textContent.trim() || style.visibility !== "visible" || box.width <= 0 || box.height <= 0) continue;
    const measured = [trimmed(box, parseFloat(style.fontSize))];
    const lines = visible(measured, text);
    if (!lines.length) continue;
    const cut = cutOf(measured, text);
    blocks.set(text, { text: text.textContent, lines, opacity: opacity(text), svg: true, cut });
  }

  const items = [];
  for (const [block, entry] of blocks) {
    if (entry.opacity < 0.01) continue;
    const bound = entry.lines.reduce((a, r) => ({
      left: Math.min(a.left, r.left), right: Math.max(a.right, r.right),
      top: Math.min(a.top, r.top), bottom: Math.max(a.bottom, r.bottom),
    }));
    // Wholly outside the frame is not seen at all: an exit, or not yet in.
    if (bound.right <= 0 || bound.left >= W || bound.bottom <= 0 || bound.top >= H) continue;
    items.push({ block, ...entry, bound, name: quote(entry.text) });
  }

  const beyond = (b, frame) => ({
    left: frame.left - b.left, right: b.right - frame.right,
    top: frame.top - b.top, bottom: b.bottom - frame.bottom,
  });
  const edge = { left: "left", right: "right", top: "top", bottom: "foot" };
  for (const item of items) {
    const off = beyond(item.bound, { left: 0, right: W, top: 0, bottom: H });
    for (const side of Object.keys(off)) {
      if (off[side] > SLACK) {
        const message = `the text ${item.name} runs off the ${edge[side]} of the frame by ${px(off[side])}`;
        findings.push({ key: message, message });
      } else if (off[side] + safe[side] > SLACK) {
        const message = `the text ${item.name} is inside the safe margin at the ${edge[side]}: ` +
          `${px(-off[side])} from the edge, where ${px(safe[side])} keeps it clear of a player's crop`;
        findings.push({ key: message, message });
      }
    }
    if (item.cut) {
      const { box, side, depth } = item.cut;
      const message = `the text ${item.name} is cut off by ${named(box)} on the ${edge[side]} by ${px(depth)}`;
      findings.push({ key: message, message });
    }
    const card = item.svg ? null : cardOf(item.block);
    if (card) {
      const over = beyond(item.bound, card.getBoundingClientRect());
      for (const side of Object.keys(over)) {
        if (over[side] <= SLACK) continue;
        const message = `the text ${item.name} overflows its box ${named(card)} on the ${edge[side]} by ${px(over[side])}`;
        findings.push({ key: message, message });
      }
    }
  }

  // Faint text behind other text is a choice (a watermark word), not a clash.
  const legible = items.filter((item) => item.opacity >= 0.25);
  for (let i = 0; i < legible.length; i++) {
    for (let j = i + 1; j < legible.length; j++) {
      const [a, b] = [legible[i], legible[j]];
      if (a.block.contains(b.block) || b.block.contains(a.block)) continue;
      const clash = a.lines.flatMap((p) => b.lines.map((q) => meet(p, q))).find((r) => r.right - r.left > 2 && r.bottom - r.top > 2);
      if (!clash) continue;
      const message = `the texts ${a.name} and ${b.name} overlap`;
      const at = [clash.left, clash.top, clash.right, clash.bottom].map(Math.round).join(",");
      findings.push({ key: `${message} @${at}`, message });
    }
  }
  return findings;
}
