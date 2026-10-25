// The draw (#1553): the same code always sees the same version, the versions
// share the visitors evenly, and two tests draw independently.

import { expect, test } from "bun:test";
import { draw } from "./draw";
import type { AbTest } from "./tests";

const heading: AbTest = { id: "heading", versions: ["a", "b"], running: true };
const strip: AbTest = { id: "film-strip", versions: ["a", "b"], running: false };
const codes = Array.from(
  { length: 4000 },
  (_, at) => `0b7f6a3e-9c1d-4e2f-8a5b-${at.toString(16).padStart(12, "0")}`,
);

test("a visitor sees the same version every time", () => {
  for (const code of codes.slice(0, 50)) expect(draw(code, heading)).toBe(draw(code, heading));
});

test("each version gets about half the visitors", () => {
  const b = codes.filter((code) => draw(code, heading) === "b").length;
  expect(b / codes.length).toBeGreaterThan(0.47);
  expect(b / codes.length).toBeLessThan(0.53);
  const three: AbTest = { id: "three", versions: ["a", "b", "c"], running: false };
  const c = codes.filter((code) => draw(code, three) === "c").length;
  expect(Math.abs(c / codes.length - 1 / 3)).toBeLessThan(0.03);
});

test("one test's version says nothing of another's", () => {
  const same = codes.filter((code) => draw(code, heading) === draw(code, strip)).length;
  expect(Math.abs(same / codes.length - 0.5)).toBeLessThan(0.03);
});
