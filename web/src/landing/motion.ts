// The landing page moves with the motion kit, and its clock is the scroll
// (#904). A section's time is how far it has come up the window (`ScrollIn.tsx`): 0
// as it starts to appear, 1 once it is whole. The kit's helpers are pure functions
// of the time they are handed, which is what lets the scroll stand in for a
// clip's clock: any scroll position is a frame of its own, and scrolling back
// plays it backwards.
//
// What moves is marked in the markup, never wired element by element:
// `data-rise="0.1"` rises in at that time and `data-emerge` comes up from
// behind an edge. A visitor who asked for less motion is handed the end of
// every timeline at once, so the page is all there and nothing moves.

import { type RefObject, useLayoutEffect } from "react";
import { kit } from "@/landing/kit";
import { prefersStill } from "@/lib/reducedMotion";

/** How long one entrance takes, in screens of scroll or in seconds. */
export interface Pace {
  length: number;
  distance: number;
}

/** Draws every marked element under `root` as it stands at time `t`. */
export function draw(root: ParentNode, t: number, pace: Pace) {
  for (const element of root.querySelectorAll<HTMLElement>("[data-rise]")) {
    const at = Number(element.dataset.rise);
    kit.rise(element, t, at, { length: pace.length, distance: pace.distance, out: 0 });
  }
  // `data-emerge` comes up from behind its parent's edge (the parent clips it).
  for (const element of root.querySelectorAll<HTMLElement>("[data-emerge]")) {
    const at = Number(element.dataset.emerge);
    const p = kit.enter(t, at, pace.length * 1.2, kit.ease.out);
    element.style.transform = `translateY(${(1 - p) * 100}%)`;
  }
}

/** The pace of a section moved by the scroll. */
export const SCROLLED: Pace = { length: 0.3, distance: 48 };
/** The pace of the hero, moved by the clock from the moment it is shown. */
const TIMED: Pace = { length: 0.9, distance: 32 };
/** How much faster than real time the hero's entrance plays: its marks and lengths are read at this speed. */
const ENTRANCE_SPEED = 1.5;

/** Runs `frame` on every scroll and resize, at most once a frame, and once before the first paint. */
export function useOnScroll(frame: () => void) {
  useLayoutEffect(() => {
    let pending = 0;
    const schedule = () => {
      if (pending) return;
      pending = requestAnimationFrame(() => {
        pending = 0;
        frame();
      });
    };
    frame();
    addEventListener("scroll", schedule, { passive: true });
    addEventListener("resize", schedule);
    return () => {
      cancelAnimationFrame(pending);
      removeEventListener("scroll", schedule);
      removeEventListener("resize", schedule);
    };
  }, [frame]);
}

/**
 * Plays the marked elements in `ref` on the clock, from the moment it mounts. Its first frame is
 * drawn before the browser paints, so the hero never shows whole for a frame and then vanishes.
 */
export function useEntrance(ref: RefObject<HTMLElement | null>) {
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    if (prefersStill()) return draw(root, Number.POSITIVE_INFINITY, TIMED);
    const start = performance.now();
    let id = 0;
    const tick = (now: number) => {
      const t = ((now - start) / 1000) * ENTRANCE_SPEED;
      draw(root, t, TIMED);
      if (t < 4) id = requestAnimationFrame(tick);
    };
    tick(start);
    return () => cancelAnimationFrame(id);
  }, [ref]);
}
