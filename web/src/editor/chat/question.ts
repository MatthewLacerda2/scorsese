// What the question card needs worked out before it draws (#710): only an
// answer with words in it can be sent, and an option is sent as its own words.

/** The answer a free-text field sends, trimmed, or `null` while it holds nothing to say. */
export function typedAnswer(text: string): string | null {
  const answer = text.trim();
  return answer === "" ? null : answer;
}

/** What the composer says while a question waits: a message sent now answers it. */
export const ANSWER_PLACEHOLDER =
  "Answer the assistant's question — pick an option above, or write your own answer here";
