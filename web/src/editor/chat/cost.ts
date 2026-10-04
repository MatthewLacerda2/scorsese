// The model picker's cost bar (#706): how dear each model is beside the
// others, never a price. The server sends each model's blended cost as a
// percentage of the dearest (`assistant::cost`); this turns that into the
// bar's colour and the words a screen reader says in its place.

import type { ModelChoice } from "@/api/chat";
import type { Messages } from "@/i18n/catalogue";

/** Where `cost` sits between the cheapest and dearest of `models`: 0 to 1. */
export function position(cost: number, models: ModelChoice[]): number {
  const costs = models.flatMap((choice) => (choice.cost === null ? [] : [choice.cost]));
  const cheapest = Math.min(...costs);
  const dearest = Math.max(...costs);
  if (!(dearest > cheapest)) return 0;
  return Math.min(1, Math.max(0, (cost - cheapest) / (dearest - cheapest)));
}

/** Green at the cheapest, through amber, to red at the dearest. A middling
 * lightness reads against both themes' backgrounds. */
export function colour(at: number): string {
  return `hsl(${Math.round(120 * (1 - at))} 70% 45%)`;
}

/** The bar in words, for a screen reader. */
export function describe(at: number, t: Messages["chat"]["cost"]): string {
  if (at <= 0) return t.cheapest;
  if (at >= 1) return t.dearest;
  if (at < 1 / 3) return t.inexpensive;
  if (at < 2 / 3) return t.moderate;
  return t.expensive;
}
