import { describe, expect, test } from "bun:test";
import type { DocumentAsset } from "@/api";
import type { QuoteView, TurnView } from "@/api/chat";
import { awaitingQuote, changeAnswer, itemKind, kindHue, offer, preview } from "./quote";

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

describe("awaitingQuote", () => {
  const quote: QuoteView = { tool: "generate", lines: [], micros: 1, expires_at: 9 };
  const turn = (fields: Partial<TurnView>) =>
    ({ state: "asking", quote, quote_answer: null, ...fields }) as TurnView;

  test("is true while the turn is paused on an unanswered quote", () => {
    expect(awaitingQuote(turn({}))).toBe(true);
  });

  test("is false once answered, and while the turn still runs", () => {
    expect(awaitingQuote(turn({ quote_answer: "confirmed" }))).toBe(false);
    expect(awaitingQuote(turn({ state: "running" }))).toBe(false);
  });

  test("is false on a turn that ended holding one: no dead box (#1440)", () => {
    expect(awaitingQuote(turn({ state: "stopped" }))).toBe(false);
  });
});

describe("itemKind", () => {
  const assets = [
    { id: "pose-wide", kind: "generated_image" },
    { id: "rain", kind: "generated_audio" },
  ] as DocumentAsset[];

  test("an asset in the document is its kind", () => {
    expect(itemKind({ subject: "pose-wide", brief: "prompt" }, assets)).toBe("generated_image");
    expect(itemKind({ subject: "rain", brief: "prompt" }, assets)).toBe("generated_audio");
  });

  test("a voice design is the speech it makes", () => {
    expect(itemKind({ subject: "design", brief: "voice" }, assets)).toBe("generated_audio");
  });

  test("a subject the document does not hold has no kind", () => {
    expect(itemKind({ subject: "gone", brief: "prompt" }, assets)).toBeUndefined();
    expect(itemKind({ subject: "pose-wide", brief: "prompt" }, undefined)).toBeUndefined();
  });
});

describe("kindHue", () => {
  test("is the kind's token, and the grey for one this build does not know", () => {
    expect(kindHue("generated_audio")).toBe("var(--kind-generated-audio)");
    expect(kindHue("hologram")).toBe("var(--kind-other)");
    expect(kindHue(undefined)).toBe("var(--kind-other)");
  });
});
