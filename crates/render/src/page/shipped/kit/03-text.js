  // An element's text cut into pieces once, on its first frame, and kept on
  // it: a span per word — a stretch between the spaces a line breaks at, so
  // its punctuation travels with it and a non-breaking space holds two words
  // together, as native text's reveal cuts it — or per grapheme, what the eye
  // reads as one character, an emoji with its skin tone or a letter with its
  // accent. A word cut into characters is held in an unbreakable span so a
  // line still breaks only between words. The spaces stay text, the text
  // stays in whatever markup held it (an `<em>` is kept), and so the element
  // reads as the same string to anything measuring it; a piece never crosses
  // a tag, so a word half in an `<em>` is two pieces.
  const cuts = new WeakMap();
  const BREAKS = /([^\S\u00a0\u2007\u202f]+)/;
  const cut = (element, unit) => {
    const by = unit === "char" ? "char" : "word";
    const known = cuts.get(element);
    if (known && known.by !== by) {
      throw new Error(`kit.reveal: this element is already cut by ${known.by}, so it cannot be revealed by ${unit} too`);
    }
    if (known) return known.pieces;
    const graphemes = new Intl.Segmenter(undefined, { granularity: "grapheme" });
    const piece = (text) => Object.assign(document.createElement("span"), { textContent: text });
    const texts = [];
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      if (!walker.currentNode.parentElement.closest("script, style")) texts.push(walker.currentNode);
    }
    const pieces = [];
    for (const node of texts) {
      node.replaceWith(...node.data.split(BREAKS).filter(Boolean).map((text, i) => {
        if (i % 2 === (BREAKS.test(node.data[0]) ? 0 : 1)) return document.createTextNode(text);
        if (by === "word") return pieces[pieces.push(piece(text)) - 1];
        const word = piece("");
        word.style.whiteSpace = "nowrap";
        for (const { segment } of graphemes.segment(text)) word.append(pieces[pieces.push(piece(segment)) - 1]);
        return word;
      }));
    }
    cuts.set(element, { by, pieces });
    return pieces;
  };

  // Word pieces gathered by the line they are laid out on, read every frame
  // from layout, which nothing here moves: a transform is not layout.
  const lines = (pieces) => pieces.reduce((found, piece) => {
    const [top, bottom] = [piece.offsetTop, piece.offsetTop + piece.offsetHeight];
    const line = found.at(-1);
    if (line && (top + bottom) / 2 < line.bottom) {
      line.pieces.push(piece);
      line.bottom = Math.max(line.bottom, bottom);
    } else found.push({ pieces: [piece], bottom });
    return found;
  }, []).map((line) => line.pieces);

  // Reveals `element`'s text a `char`, `word` or `line` at a time from `at`:
  // each piece `stagger` seconds after the one before (half its `length` by
  // default, a cascade, as native reveal's default), fading in over `length`
  // seconds eased `by` as it rises `rise` px into place (a fifth of the
  // element's font size by default, as native's; 0 fades in place). Answers
  // when the last piece lands, so the next thing can start after it.
  const reveal = (element, t, at = 0, { unit = "word", stagger: step, length = 0.5, rise, by = ease.out } = {}) => {
    if (!["char", "word", "line"].includes(unit)) {
      throw new Error(`kit.reveal: unit ${unit} is none of char, word or line`);
    }
    const pieces = cut(element, unit);
    const lift = rise ?? 0.2 * parseFloat(getComputedStyle(element).fontSize);
    const gap = step ?? length / 2;
    for (const piece of pieces) piece.style.display = lift ? "inline-block" : "";
    const groups = unit === "line" ? lines(pieces) : pieces.map((piece) => [piece]);
    groups.forEach((group, i) => {
      const p = remap(t, at + i * gap, at + i * gap + length, 0, 1, by);
      for (const piece of group) {
        piece.style.opacity = clamp(p);
        piece.style.transform = lift ? `translateY(${(1 - p) * lift}px)` : "";
      }
    });
    return at + Math.max(0, groups.length - 1) * gap + length;
  };

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

