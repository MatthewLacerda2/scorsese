// The word a working turn shows (#767): one of the language's playful status
// words, changing every few seconds, never the same twice in a row. The
// model's thinking is not shown — nobody read it — so this is all that says
// the assistant is alive while it works.

/** How long one word stays, in milliseconds. */
export const EVERY_MS = 3500;

/**
 * A word from `words` other than `previous` (when there is another), chosen
 * by `random` — `Math.random`, or a test's fixed sequence.
 */
export function nextWord(
  words: readonly string[],
  previous: string | null,
  random: () => number = Math.random,
): string {
  const choices = words.length > 1 ? words.filter((word) => word !== previous) : words;
  return choices[Math.floor(random() * choices.length)] ?? choices[0] ?? "";
}
