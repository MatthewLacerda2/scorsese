import { describe, expect, test } from "bun:test";
import { changeAnswer, preview } from "./quote";

describe("preview", () => {
  test("keeps a short description whole", () => {
    expect(preview("A red cape.", 20)).toEqual({ shown: "A red cape.", clipped: false });
  });

  test("cuts a long one at a word", () => {
    const { shown, clipped } = preview("a superhero in a red cape lands on a roof", 20);
    expect(clipped).toBe(true);
    expect(shown).toBe("a superhero in a red…");
  });

  test("cuts mid-word when there is no space worth cutting at", () => {
    expect(preview("abcdefghijklmnopqrstuvwxyz", 10).shown).toBe("abcdefghij…");
  });
});

describe("changeAnswer", () => {
  test("sends the words, trimmed, as a no with a change", () => {
    expect(changeAnswer("  make the cape yellow ")).toEqual({
      confirm: false,
      change: "make the cape yellow",
    });
  });

  test("sends nothing for an empty field", () => {
    expect(changeAnswer("   ")).toBeNull();
  });
});
