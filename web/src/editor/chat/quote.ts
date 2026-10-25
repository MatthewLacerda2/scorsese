// What the quote box needs worked out before it draws (#709): a long prompt is
// shown collapsed, only a change with words in it can be sent, and a half-price
// batch offered beside the price for now (#947) is a second yes, the box
// shows only while its turn is paused on it (#1440), and each item's name is
// drawn in its asset kind's hue, the timeline's (#1573).

import type { DocumentAsset } from "@/api";
import type { QuoteAnswer, QuoteItem, QuoteView, TurnView } from "@/api/chat";
import { kindColor } from "../assets/kinds";

/** How much of a description shows before the expander. */
const PREVIEW_CHARS = 160;

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

/**
 * True while a turn's quote waits for the user's yes or no: only a paused turn's box can
 * be answered, so a turn that ended any other way shows none (#1440).
 */
export function awaitingQuote(turn: TurnView): boolean {
  return turn.state === "asking" && turn.quote !== null && turn.quote_answer === null;
}

/**
 * The asset kind a quoted item makes: its asset's, from the project document,
 * where it waits as a sketch; a voice design's (`design`, no asset) is the
 * speech it makes. `undefined` for a subject the document does not hold.
 */
export function itemKind(
  item: Pick<QuoteItem, "subject" | "brief">,
  assets: DocumentAsset[] | undefined,
): string | undefined {
  const asset = assets?.find((held) => held.id === item.subject);
  return asset?.kind ?? (item.brief === "voice" ? "generated_audio" : undefined);
}

/**
 * The kind's hue as a CSS colour: the `--kind-*` token behind `kindColor`'s
 * class, so a name is drawn in the colour its clip has on the timeline.
 */
export function kindHue(kind: string | undefined): string {
  return `var(--${kindColor(kind).slice("bg-".length)})`;
}
