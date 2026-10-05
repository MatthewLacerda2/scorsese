// Scorsese owns this page's clock (#594, #606).
//
// Run before any of the page's own scripts. Every clock a page can read —
// performance.now(), Date, setTimeout/setInterval, requestAnimationFrame, and
// every Web Animation (CSS animations and transitions included) — stands still
// until the capture calls __scorsese.advanceTo(ms), and then moves exactly that
// far. So a frame is a pure function of its time, which is what lets a capture
// be cached, compared with a golden, and reproduced on another machine.
//
// One advance runs, in this order:
//   1. every timer due by then, earliest first, the clock set to each one's
//      own due time as it fires (an interval re-arms itself and can fire again);
//   2. every animation frame requested so far, all with the target time;
//   3. every Web Animation paused and seeked to how long it has existed.
// Timers before frames because a timer is what usually starts the animation a
// frame then draws; seeking last because frame callbacks can create animations.
//
// The clock starts at 100 ms rather than 0: anime.js reads a frame timestamp of
// 0 as "not started yet" (#606).
//
// Not covered, and said in docs/project-format.md: workers' clocks,
// requestIdleCallback, and <video>/<audio>, which play on their own clocks.
(() => {
  const START = 100;
  const EPOCH = 1700000000000;
  let now = START;
  let nextId = 1;
  const timers = new Map();
  let frames = new Map();
  const born = new WeakMap();

  performance.now = () => now;
  const RealDate = Date;
  window.Date = class extends RealDate {
    constructor(...args) {
      if (args.length) super(...args);
      else super(EPOCH + now);
    }
    static now() {
      return EPOCH + now;
    }
  };

  const arm = (fn, ms, args, interval) => {
    const id = nextId++;
    timers.set(id, { due: now + ms, fn, args, interval });
    return id;
  };
  window.setTimeout = (fn, ms = 0, ...args) => arm(fn, Math.max(0, +ms || 0), args, null);
  window.setInterval = (fn, ms = 0, ...args) => {
    const every = Math.max(1, +ms || 0);
    return arm(fn, every, args, every);
  };
  window.clearTimeout = window.clearInterval = (id) => timers.delete(id);
  window.requestAnimationFrame = (fn) => {
    const id = nextId++;
    frames.set(id, fn);
    return id;
  };
  window.cancelAnimationFrame = (id) => frames.delete(id);

  // A callback that throws must not stop the clock: it is reported, the way an
  // uncaught error in a real timer would be, and the advance carries on.
  const call = (fn, args) => {
    try {
      if (typeof fn === "function") fn(...args);
      else (0, eval)(String(fn));
    } catch (error) {
      reportError(error);
    }
  };

  const advanceTo = (ms) => {
    const target = START + ms;
    for (;;) {
      let next = null;
      for (const [id, timer] of timers) {
        if (timer.due <= target && (!next || timer.due < next[1].due)) next = [id, timer];
      }
      if (!next) break;
      const [id, timer] = next;
      now = timer.due;
      if (timer.interval) timer.due += timer.interval;
      else timers.delete(id);
      call(timer.fn, timer.args);
    }
    now = target;
    const due = frames;
    frames = new Map();
    for (const fn of due.values()) call(fn, [now]);
    for (const animation of document.getAnimations()) {
      if (!born.has(animation)) born.set(animation, now);
      animation.pause();
      animation.currentTime = now - born.get(animation);
    }
  };

  Object.defineProperty(window, "__scorsese", {
    value: Object.freeze({ advanceTo }),
    enumerable: false,
  });
})();
