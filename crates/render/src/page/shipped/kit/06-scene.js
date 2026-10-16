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
    frame, clamp, lerp, remap, ease, enter, exit, rise, stagger, reveal, count, random, svg,
    draw, erase, along, font, write, camera, person,
  });
})();
