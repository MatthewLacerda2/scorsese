// Gesture zoom: it clamps, it treats a pinch and a wheel notch alike, and the
// moment under the pointer stays under it.

import { describe, expect, test } from "bun:test";
import { keptUnder, MAX_ZOOM, MIN_ZOOM, zoomed } from "./zoom";

describe("zoom", () => {
  test("pinching out zooms in, pinching in zooms out", () => {
    expect(zoomed({ pxPerSecond: 40 }, -10).pxPerSecond).toBeGreaterThan(40);
    expect(zoomed({ pxPerSecond: 40 }, 10).pxPerSecond).toBeLessThan(40);
  });

  test("a mouse notch counts as much as one, in pixels or in lines", () => {
    const notch = zoomed({ pxPerSecond: 40 }, 100).pxPerSecond;
    expect(notch).toBeCloseTo(zoomed({ pxPerSecond: 40 }, 50).pxPerSecond);
    expect(zoomed({ pxPerSecond: 40 }, 4, 1).pxPerSecond).toBeCloseTo(notch);
  });

  test("never past the widest or the closest", () => {
    expect(zoomed({ pxPerSecond: MAX_ZOOM }, -50).pxPerSecond).toBe(MAX_ZOOM);
    expect(zoomed({ pxPerSecond: MIN_ZOOM }, 50).pxPerSecond).toBe(MIN_ZOOM);
  });

  test("the moment under the pointer stays under it", () => {
    const from = { pxPerSecond: 40 };
    const to = { pxPerSecond: 80 };
    // 300px scrolled, pointer 100px in: 400px is 10 s at 40 px/s.
    const scroll = keptUnder(100, 300, from, to);
    expect((scroll + 100) / to.pxPerSecond).toBe(10);
    // Zooming out near the head never scrolls before it.
    expect(keptUnder(100, 0, to, from)).toBe(0);
  });
});
