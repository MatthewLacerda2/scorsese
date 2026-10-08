import { describe, expect, test } from "bun:test";
import type { QuoteView } from "@/api/chat";
import { changeAnswer, offer, preview } from "./quote";

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

describe("offer", () => {
  const quote: QuoteView = { tool: "generate", lines: [], micros: 1_320_000, expires_at: 0 };

  test("offers both prices when a batch is beside the quote", () => {
    expect(offer({ ...quote, batch_micros: 660_000 })).toEqual({ now: 1_320_000, batch: 660_000 });
  });

  test("offers one price otherwise, including on a quote held before batches", () => {
    expect(offer(quote)).toBeNull();
    expect(offer({ ...quote, batch_micros: null })).toBeNull();
  });
});
