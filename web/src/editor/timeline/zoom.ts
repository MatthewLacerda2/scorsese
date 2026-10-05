// Zooming the timeline by gesture: a trackpad's pinch and Ctrl + the mouse
// wheel, centred on the pointer — what every editor does, and what freed the
// toolbar of its − / + buttons (#765).
//
// Browsers deliver a pinch as a `wheel` event with `ctrlKey` set, so the two
// gestures are one code path. A pinch sends many small deltas and a mouse
// notch one large one; scaling exponentially by the delta, with a notch's
// delta capped, makes both feel like one zoom.

import type { Zoom } from "./time";

/** The widest and the closest the timeline zooms: a whole minute on screen, and single frames. */
export const MIN_ZOOM = 10;
export const MAX_ZOOM = 320;
/** Where a timeline opens. */
export const START_ZOOM = 40;

/** How much one pixel of wheel travel zooms by, as an exponent. */
const PER_PX = 0.01;
/** The most one wheel event counts for, in pixels: one notch of a mouse wheel. */
const NOTCH = 50;
/** A line-mode wheel's line, in pixels (Firefox counts a mouse notch in lines). */
const LINE = 16;

/** The zoom after a wheel of `deltaY` (positive is away from the user: out),
 * clamped to [MIN_ZOOM, MAX_ZOOM]. `deltaMode` is the event's own: 0 pixels,
 * 1 lines. */
export function zoomed(zoom: Zoom, deltaY: number, deltaMode = 0): Zoom {
  const px = deltaMode === 1 ? deltaY * LINE : deltaY;
  const travel = Math.max(-NOTCH, Math.min(NOTCH, px));
  const next = zoom.pxPerSecond * Math.exp(-travel * PER_PX);
  return { pxPerSecond: Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, next)) };
}

/** The scroll that keeps the moment under the pointer where it was, after a
 * zoom from `from` to `to`. `pointer` is how far the pointer is from the start
 * of the lanes on screen (past the lane headers), `scroll` how far the
 * timeline is scrolled now. Never before the head of the timeline. */
export function keptUnder(pointer: number, scroll: number, from: Zoom, to: Zoom): number {
  const at = (scroll + pointer) / from.pxPerSecond;
  return Math.max(0, at * to.pxPerSecond - pointer);
}
