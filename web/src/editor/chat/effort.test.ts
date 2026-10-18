// The four levels, cheapest first, and anything else read as a new project's.

import { expect, test } from "bun:test";
import { EFFORTS, effortOf } from "./effort";

test("four levels, extra being the API's xhigh", () => {
  expect([...EFFORTS]).toEqual(["low", "medium", "high", "xhigh"]);
  for (const level of EFFORTS) expect(effortOf(level)).toBe(level);
});

test("a level the server never sends reads as high", () => {
  expect(effortOf(undefined)).toBe("high");
  expect(effortOf("max")).toBe("high");
});
