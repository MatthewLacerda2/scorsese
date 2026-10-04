// Money as the user reads it: US dollars, and nothing else (#703). The ledger
// is integer micro-dollars and never a float (docs/web.md, *Money*): one
// assistant call can cost a fraction of a cent, and a float would round it
// away. So dollars are formatted by integer arithmetic, digit by digit.

const MICROS_PER_DOLLAR = 1_000_000;
const MINUS = "−";

const wholeDollars = new Intl.NumberFormat("en-US");

/**
 * Micro-dollars as dollars: at least two decimals, and as many more as the
 * amount needs, so `4_200` is `$0.0042` rather than a misleading `$0.00`.
 */
export function formatDollars(micros: number): string {
  const sign = micros < 0 ? MINUS : "";
  const magnitude = Math.abs(Math.trunc(micros));
  const whole = Math.floor(magnitude / MICROS_PER_DOLLAR);
  const fraction = String(magnitude % MICROS_PER_DOLLAR)
    .padStart(6, "0")
    .replace(/0+$/, "")
    .padEnd(2, "0");
  return `${sign}$${wholeDollars.format(whole)}.${fraction}`;
}

/**
 * A ledger movement with its direction spelled out: `+` for money in, `−` for
 * money out, nothing for zero (a generation the provider failed is free).
 */
export function formatMovement(micros: number): string {
  if (micros === 0) return formatDollars(0);
  const sign = micros > 0 ? "+" : MINUS;
  return `${sign}${formatDollars(Math.abs(micros))}`;
}
