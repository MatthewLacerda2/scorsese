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

