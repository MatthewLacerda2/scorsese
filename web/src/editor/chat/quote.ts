// What the quote box needs worked out before it draws (#709): a long prompt is
// shown collapsed, only a change with words in it can be sent, and a half-price
// batch offered beside the price for now (#947) is a second yes.

import type { QuoteAnswer, QuoteView } from "@/api/chat";

/** How much of a description shows before the expander. */
export const PREVIEW_CHARS = 160;

/** `text` as shown collapsed: whole when short, else cut at a word with an ellipsis. */
export function preview(text: string, limit = PREVIEW_CHARS): { shown: string; clipped: boolean } {
  if (text.length <= limit) return { shown: text, clipped: false };
  const cut = text.slice(0, limit);
  const space = text[limit] === " " ? limit : cut.lastIndexOf(" ");
  const shown = (space > limit / 2 ? cut.slice(0, space) : cut).trimEnd();
  return { shown: `${shown}…`, clipped: true };
}

/** The answer a change field sends, or `null` while it holds nothing to say. */
export function changeAnswer(text: string): QuoteAnswer | null {
  const change = text.trim();
  return change ? { confirm: false, change } : null;
}

/**
 * The two prices a quote offers, in micro-dollars — now, and in the half-price
 * batch beside it (#947) — or `null` when it offers only the one.
 */
export function offer(quote: QuoteView): { now: number; batch: number } | null {
  const batch = quote.batch_micros;
  return batch ? { now: quote.micros, batch } : null;
}
