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
    const lines = [...range.getClientRects()].filter((r) => r.width > 0 && r.height > 0).map((r) => trimmed(r, size));
    if (!lines.length) continue;
    let block = parent;
    while (!isBox(block) && block.parentElement) block = block.parentElement;
    const entry = blocks.get(block) || { text: "", lines: [], opacity: opacity(parent), svg: false };
    entry.text += node.data;
    entry.lines.push(...lines);
    entry.opacity = Math.max(entry.opacity, opacity(parent));
    blocks.set(block, entry);
  }
  for (const text of document.querySelectorAll("svg text")) {
    if (text.parentElement && text.parentElement.closest("text")) continue;
    const style = getComputedStyle(text);
    const box = text.getBoundingClientRect();
    if (!text.textContent.trim() || style.visibility !== "visible" || box.width <= 0 || box.height <= 0) continue;
    const line = trimmed(box, parseFloat(style.fontSize));
    blocks.set(text, { text: text.textContent, lines: [line], opacity: opacity(text), svg: true });
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
      const clash = a.lines.flatMap((p) => b.lines.map((q) => ({
        left: Math.max(p.left, q.left), right: Math.min(p.right, q.right),
        top: Math.max(p.top, q.top), bottom: Math.min(p.bottom, q.bottom),
      }))).find((r) => r.right - r.left > 2 && r.bottom - r.top > 2);
      if (!clash) continue;
      const message = `the texts ${a.name} and ${b.name} overlap`;
      const at = [clash.left, clash.top, clash.right, clash.bottom].map(Math.round).join(",");
      findings.push({ key: `${message} @${at}`, message });
    }
  }
  return findings;
}
