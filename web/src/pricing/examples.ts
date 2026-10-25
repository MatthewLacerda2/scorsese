// What things cost, as the user pays them, and the plans on the pricing page.
// These figures are written here for now; they will come from a prices
// endpoint on the server, and this module is the only place the page reads
// them from, so that change is one file. Micro-dollars, as the ledger keeps
// them (`lib/money.ts`).

/** A real example of what something costs: one amount, or a range. */
export interface CostExample {
  id: "image" | "narration" | "music" | "video" | "assistant";
  fromMicros: number;
  toMicros?: number;
}

export const COST_EXAMPLES: readonly CostExample[] = [
  { id: "image", fromMicros: 110_000 },
  { id: "narration", fromMicros: 20_000, toMicros: 40_000 },
  { id: "music", fromMicros: 180_000 },
  { id: "video", fromMicros: 1_440_000 },
  { id: "assistant", fromMicros: 30_000, toMicros: 300_000 },
];

/** The free credit: every day, up to a monthly cap, no card. */
export const FREE = { dailyMicros: 4_000_000, monthlyMicros: 12_000_000 };

/** The purchase sizes in reais, and the bonus credit each adds, in percent. */
export const PLANS = [
  { reais: 20, bonusPercent: 0, best: false },
  { reais: 40, bonusPercent: 5, best: false },
  { reais: 80, bonusPercent: 10, best: true },
] as const;
