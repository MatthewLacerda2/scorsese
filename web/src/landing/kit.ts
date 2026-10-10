// scorsese's motion kit (#812), the very file a page's video is drawn with,
// loaded here rather than copied so the two never drift. It is a classic
// script that sets `window.kit`; importing it for its side effect runs it once.
//
// Only the helpers that are pure functions of the time they are handed are
// used on the web: `kit.frame` is tied to a page's capture clock and `kit.exit`
// to its `scorsese.duration`, neither of which a browser tab has.
import "../../../crates/render/src/page/shipped/kit.js";

/** An easing: progress 0–1 in, eased progress out. */
type Ease = (p: number) => number;

/** The part of the kit the landing page uses. */
export interface Kit {
  clamp: (x: number, lo?: number, hi?: number) => number;
  lerp: (a: number, b: number, p: number) => number;
  remap: (x: number, a: number, b: number, c?: number, d?: number, by?: Ease) => number;
  ease: { linear: Ease; in: Ease; out: Ease; inOut: Ease; back: Ease };
  enter: (t: number, at?: number, length?: number, by?: Ease) => number;
  rise: (
    element: HTMLElement,
    t: number,
    at?: number,
    options?: { length?: number; distance?: number; out?: number },
  ) => void;
  stagger: (index: number, step?: number, first?: number) => number;
  count: (
    t: number,
    at: number,
    length: number,
    to: number,
    options?: { from?: number; decimals?: number; locale?: string },
  ) => string;
}

export const kit = (window as unknown as { kit: Kit }).kit;
