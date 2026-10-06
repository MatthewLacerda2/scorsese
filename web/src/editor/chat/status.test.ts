// The status word never repeats itself, and every language has its own (#767).

import { expect, test } from "bun:test";
import { CATALOGUES } from "@/i18n/catalogue";
import { LANGUAGES } from "@/i18n/language";
import { nextWord } from "./status";

test("the next word is never the one before, whatever the dice say", () => {
  const words = ["Splicing…", "Rewinding the tape…", "Lining up the cuts…"];
  for (const roll of [0, 0.34, 0.67, 0.999]) {
    for (const previous of words) {
      const next = nextWord(words, previous, () => roll);
      expect(words).toContain(next);
      expect(next).not.toBe(previous);
    }
  }
});

test("a first word, and a list of one, still give a word", () => {
  expect(nextWord(["Splicing…"], null, () => 0.5)).toBe("Splicing…");
  expect(nextWord(["Splicing…"], "Splicing…", () => 0.5)).toBe("Splicing…");
  expect(nextWord(["a", "b"], null, () => 0.99)).toBe("b");
});

test.each(LANGUAGES.map(({ language }) => language))("%s has its own status words", (language) => {
  const words = CATALOGUES[language].chat.turn.busy;
  expect(words.length).toBeGreaterThanOrEqual(8);
  expect(new Set(words).size).toBe(words.length);
  for (const word of words) expect(word.trim()).not.toBe("");
});
