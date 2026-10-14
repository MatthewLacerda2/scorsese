// The slides' layout: each one's stretch of the scroll, and where it stands at
// a given scroll position — before, during and after its crossfades.

import { expect, test } from "bun:test";
import { COMMIT, CROSSFADE, frameAt, layout, restingPoint } from "@/landing/slideLayout";

test("a slide that fits the window takes one crossfade of scroll; a taller one, its overflow too", () => {
  const { spans, total, fade } = layout([800, 1500, 600], 1000);
  expect(fade).toBe(1000 * CROSSFADE);
  expect(spans).toEqual([
    { start: 0, extra: 0 },
    { start: fade, extra: 500 },
    { start: fade + 500 + fade, extra: 0 },
  ]);
  // The last slide's top, plus a window to see it in.
  expect(total).toBe(fade + 500 + fade + 1000);
  expect(layout([], 1000).total).toBe(0);
});

test("a slide fades in over the crossfade before it, and out over the one after", () => {
  const span = { start: 600, extra: 0 };
  expect(frameAt(span, 0, 600).opacity).toBe(0);
  expect(frameAt(span, 300, 600).opacity).toBe(0.5);
  expect(frameAt(span, 600, 600).opacity).toBe(1);
  expect(frameAt(span, 900, 600).opacity).toBe(0.5);
  expect(frameAt(span, 1200, 600).opacity).toBe(0);
});

test("a taller slide stays whole while it scrolls through itself, then fades", () => {
  const span = { start: 600, extra: 500 };
  expect(frameAt(span, 600, 600)).toEqual({ shift: 0, opacity: 1, time: 1 });
  expect(frameAt(span, 850, 600).shift).toBe(250);
  expect(frameAt(span, 1100, 600)).toEqual({ shift: 500, opacity: 1, time: 1 });
  // Past its end it holds still and fades.
  expect(frameAt(span, 1400, 600)).toEqual({ shift: 500, opacity: 0.5, time: 1 });
  // Eased: a quarter of the way out it has lost less than a quarter.
  expect(frameAt(span, 1250, 600).opacity).toBeGreaterThan(0.75);
});

test("a slide's entrance clock runs 0 to 1 across the crossfade that brings it in", () => {
  const span = { start: 600, extra: 0 };
  expect(frameAt(span, 0, 600).time).toBe(0);
  expect(frameAt(span, 300, 600).time).toBe(0.5);
  expect(frameAt(span, 2000, 600).time).toBe(1);
});

test("let go partway, a crossfade goes on once a quarter in, and back otherwise", () => {
  const spans = [
    { start: 0, extra: 0 },
    { start: 600, extra: 0 },
  ];
  const quarter = 600 * COMMIT;
  // Going down: past a quarter it goes on, short of it it goes back.
  expect(restingPoint(spans, 600, quarter + 1, 1)).toBe(600);
  expect(restingPoint(spans, 600, quarter - 1, 1)).toBe(0);
  // Going up, the same quarter counted from the other end.
  expect(restingPoint(spans, 600, 600 - quarter - 1, -1)).toBe(0);
  expect(restingPoint(spans, 600, 600 - quarter + 1, -1)).toBe(600);
  // Resting on a slide, nothing to finish.
  expect(restingPoint(spans, 600, 0, 1)).toBeNull();
  expect(restingPoint(spans, 600, 600, 1)).toBeNull();
});
