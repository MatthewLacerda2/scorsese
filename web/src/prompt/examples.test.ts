// The placeholder examples are read at a glance as they type: none is longer
// than 50 characters, in any language, and each language has several.

import { expect, test } from "bun:test";
import { CATALOGUES } from "@/i18n/catalogue";

const MAX = 50;

test("every example prompt, in every language, is at most 50 characters", () => {
  const tooLong = Object.entries(CATALOGUES).flatMap(([language, messages]) =>
    messages.prompt.examples
      .filter((example) => [...example].length > MAX)
      .map((example) => `${language} (${[...example].length}): ${example}`),
  );
  expect(tooLong).toEqual([]);
});

test("each language has a handful of examples", () => {
  for (const messages of Object.values(CATALOGUES)) {
    expect(messages.prompt.examples.length).toBeGreaterThanOrEqual(6);
  }
});
