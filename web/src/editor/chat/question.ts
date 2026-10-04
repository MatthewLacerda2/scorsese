// What the question card needs worked out before it draws (#710): only an
// answer with words in it can be sent, and an option is sent as its own words.

/** The answer a free-text field sends, trimmed, or `null` while it holds nothing to say. */
export function typedAnswer(text: string): string | null {
  const answer = text.trim();
  return answer === "" ? null : answer;
}
