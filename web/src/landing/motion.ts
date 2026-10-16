// The landing page moves with the motion kit, and its clock is the scroll
// (#904). A slide's time is how far it has faded in (`slideLayout.ts`): 0 as it
// starts to appear, 1 once it is whole. The kit's helpers are pure functions
// of the time they are handed, which is what lets the scroll stand in for a
// clip's clock: any scroll position is a frame of its own, and scrolling back
// plays it backwards.
//
// What moves is marked in the markup, never wired element by element:
// `data-rise="0.1"` rises in at that time, `data-emerge` comes up from behind
// an edge, `data-count="16"` counts up to its
// figure from `data-at`. A visitor who asked for less motion is handed the end
// of every timeline at once, so the page is all there and nothing moves.

import { type RefObject, useEffect } from "react";
import { kit } from "@/landing/kit";

/** How long one entrance takes, in screens of scroll or in seconds. */
export interface Pace {
  length: number;
  distance: number;
}

/** Whether the visitor asked their system for less motion. */
export function prefersStill(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Draws every marked element under `root` as it stands at time `t`. */
export function draw(root: ParentNode, t: number, pace: Pace, locale: string) {
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
  for (const element of root.querySelectorAll<HTMLElement>("[data-count]")) {
    const at = Number(element.dataset.at ?? 0);
    const to = Number(element.dataset.count);
    const decimals = Number(element.dataset.decimals ?? 0);
    element.textContent = kit.count(t, at, pace.length * 2, to, { decimals, locale });
  }
}

/** The pace of a section moved by the scroll. */
export const SCROLLED: Pace = { length: 0.3, distance: 48 };
/** The pace of the hero, moved by the clock from the moment it is shown. */
const TIMED: Pace = { length: 0.9, distance: 32 };
/** How much faster than real time the hero's entrance plays: its marks and lengths are read at this speed. */
const ENTRANCE_SPEED = 1.5;

/** Runs `frame` on every scroll and resize, at most once a frame, and once at the start. */
export function useOnScroll(frame: () => void) {
  useEffect(() => {
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

/** Plays the marked elements in `ref` on the clock, from the moment it mounts. */
export function useEntrance(ref: RefObject<HTMLElement | null>, locale: string) {
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    if (prefersStill()) return draw(root, Number.POSITIVE_INFINITY, TIMED, locale);
    const start = performance.now();
    let id = 0;
    const tick = (now: number) => {
      const t = ((now - start) / 1000) * ENTRANCE_SPEED;
      draw(root, t, TIMED, locale);
      if (t < 4) id = requestAnimationFrame(tick);
    };
    tick(start);
    return () => cancelAnimationFrame(id);
  }, [ref, locale]);
}
