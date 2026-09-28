// A click picks one clip; a modified click gathers several.

import { describe, expect, test } from "bun:test";
import { adds, choose, kept } from "./selection";

describe("the selection", () => {
  test("a plain click picks that clip alone", () => {
    expect(choose(["a", "b"], "c", false)).toEqual(["c"]);
    expect(choose(["a", "b"], "a", false)).toEqual(["a"]);
  });

  test("a modified click adds a clip, or takes it out again", () => {
    expect(choose(["a"], "b", true)).toEqual(["a", "b"]);
    expect(choose(["a", "b"], "a", true)).toEqual(["b"]);
  });

  test("Shift, Ctrl and ⌘ each add", () => {
    const none = { shiftKey: false, ctrlKey: false, metaKey: false };
    expect(adds(none)).toBe(false);
    expect(adds({ ...none, shiftKey: true })).toBe(true);
    expect(adds({ ...none, ctrlKey: true })).toBe(true);
    expect(adds({ ...none, metaKey: true })).toBe(true);
  });

  test("clips the project lost are let go, and an unchanged selection is the same one", () => {
    const current = ["a", "b"];
    expect(kept(current, new Set(["b", "c"]))).toEqual(["b"]);
    expect(kept(current, new Set(["a", "b"]))).toBe(current);
  });
});
