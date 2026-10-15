// `clamp`, the one way the front-end holds a number between two bounds.

import { expect, it } from "bun:test";
import { clamp } from "@/lib/utils";

it("holds a value inside its bounds and lets the lower one win if they cross", () => {
  expect(clamp(5, 0, 10)).toBe(5);
  expect(clamp(-3, 0, 10)).toBe(0);
  expect(clamp(12, 0, 10)).toBe(10);
  expect(clamp(7, 0, -1)).toBe(0);
});
