// The slides' layout: each one's stretch of the scroll, and where it stands at
// a given scroll position — before, during and after its crossfades.

import { expect, test } from "bun:test";
import { CROSSFADE, frameAt, layout } from "@/landing/slideLayout";

test("a slide that fits the window takes one crossfade of scroll; a taller one, its overflow too", () => {
  const { spans, total, fade } = layout([800, 1500, 600], 1000);
  expect(fade).toBe(1000 * CROSSFADE);
  expect(spans).toEqual([
    { start: 0, extra: 0 },
    { start: 600, extra: 500 },
    { start: 1700, extra: 0 },
  ]);
  // The last slide's top, plus a window to see it in.
  expect(total).toBe(1700 + 1000);
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
});

test("a slide's entrance clock runs 0 to 1 across the crossfade that brings it in", () => {
  const span = { start: 600, extra: 0 };
  expect(frameAt(span, 0, 600).time).toBe(0);
  expect(frameAt(span, 300, 600).time).toBe(0.5);
  expect(frameAt(span, 2000, 600).time).toBe(1);
});
