import { describe, expect, test } from "bun:test";
import { clampFrame } from "./still";

describe("clampFrame", () => {
  test("leaves a frame inside the timeline alone", () => {
    expect(clampFrame(0, 450)).toBe(0);
    expect(clampFrame(449, 450)).toBe(449);
  });
  test("shows the last frame when the playhead rests at the end or past it", () => {
    expect(clampFrame(450, 450)).toBe(449);
    expect(clampFrame(453, 450)).toBe(449);
  });
  test("never goes below frame zero", () => {
    expect(clampFrame(3, 0)).toBe(0);
  });
});
