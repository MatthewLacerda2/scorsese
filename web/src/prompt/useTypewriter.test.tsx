// The placeholder's life: empty, typed, held, erased, the next one typed;
// quiet while the box has text; and standing still for reduced motion.

import { afterEach, beforeEach, expect, jest, test } from "bun:test";
import { act, renderHook } from "@testing-library/react";
import { PACE, useTypewriter } from "@/prompt/useTypewriter";

const EXAMPLES = ["abc", "xy"];
const exact = () => 0.5;
const real = window.matchMedia;

beforeEach(() => jest.useFakeTimers());
afterEach(() => {
  jest.useRealTimers();
  window.matchMedia = real;
});

const advance = (ms: number) => act(() => void jest.advanceTimersByTime(ms));

test("it starts empty, types the first example, holds it, erases it, and types the next", () => {
  const { result } = renderHook(() => useTypewriter(EXAMPLES, true, exact));
  expect(result.current).toBe("");
  advance(PACE.start);
  expect(result.current).toBe("a");
  advance(PACE.type);
  expect(result.current).toBe("ab");
  advance(PACE.type);
  expect(result.current).toBe("abc");
  advance(PACE.hold - 1);
  expect(result.current).toBe("abc");
  advance(1);
  expect(result.current).toBe("ab");
  advance(PACE.erase * 2);
  expect(result.current).toBe("");
  advance(PACE.gap);
  expect(result.current).toBe("x");
  advance(PACE.type);
  expect(result.current).toBe("xy");
});

test("it goes round to the first example again after the last", () => {
  const { result } = renderHook(() => useTypewriter(EXAMPLES, true, exact));
  advance(PACE.start + PACE.type * 2 + PACE.hold + PACE.erase * 3 + PACE.gap);
  advance(PACE.type + PACE.hold + PACE.erase * 2 + PACE.gap);
  advance(0);
  expect(result.current).toBe("a");
});

test("typing is slower than erasing", () => {
  expect(PACE.type).toBeGreaterThan(PACE.erase);
});

test("while the box has text no timer runs, and it begins again when emptied", () => {
  const { result, rerender } = renderHook(({ active }) => useTypewriter(EXAMPLES, active, exact), {
    initialProps: { active: true },
  });
  advance(PACE.start);
  expect(result.current).toBe("a");
  rerender({ active: false });
  expect(jest.getTimerCount()).toBe(0);
  rerender({ active: true });
  expect(result.current).toBe("");
  advance(PACE.start);
  expect(result.current).toBe("a");
});

test("asked for less motion, it shows the first example and sets no timer", () => {
  window.matchMedia = ((query: string) => ({ matches: query.includes("reduce") })) as never;
  const { result } = renderHook(() => useTypewriter(EXAMPLES, true, exact));
  expect(result.current).toBe("abc");
  expect(jest.getTimerCount()).toBe(0);
});
