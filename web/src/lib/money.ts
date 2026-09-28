// Money as the user reads it. The ledger is integer micro-dollars and never a
// float (docs/web.md, *Money*): one assistant call can cost a fraction of a
// cent, and a float would round it away. So dollars are formatted by integer
// arithmetic, digit by digit, and the reais figure is the server's own
// conversion (centavos at the operator's dated rate), shown with "≈" because
// a balance in dollars drifts with the exchange rate.

const MICROS_PER_DOLLAR = 1_000_000;
const MINUS = "−";

const reais = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });
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

/** Centavos as reais, the Brazilian way: `R$ 1.234,56`. */
export function formatReais(centavos: number): string {
  const text = reais.format(Math.abs(centavos) / 100);
  return centavos < 0 ? `${MINUS}${text}` : text;
}

/**
 * An amount for a label: "≈ R$ 12,34" when the server converted it, dollars
 * when no display rate is set yet.
 */
export function formatMoney(micros: number, centavos: number | null): string {
  return centavos === null ? formatDollars(micros) : `≈ ${formatReais(centavos)}`;
}

/**
 * Micro-dollars in centavos at a display rate (reais per dollar, in
 * ten-thousandths), rounded half away from zero — the same arithmetic as the
 * server's `DisplayRate::centavos`, for the amounts it sends in dollars only
 * (a generation record's cost). BigInt because micros × rate outgrows a
 * float's exact integers long before a balance is unusual.
 */
export function toCentavos(micros: number, brlPerUsdE4: number): number {
  const exact = BigInt(Math.trunc(micros)) * BigInt(Math.trunc(brlPerUsdE4));
  const half = exact < 0n ? -50_000_000n : 50_000_000n;
  return Number((exact + half) / 100_000_000n);
}

/**
 * A ledger movement with its direction spelled out: `+` for money in, `−` for
 * money out, nothing for zero (a generation the provider failed is free).
 */
export function formatMovement(micros: number, centavos: number | null): string {
  if (micros === 0) return formatMoney(0, centavos === null ? null : 0);
  const sign = micros > 0 ? "+" : MINUS;
  return centavos === null
    ? `${sign}${formatDollars(Math.abs(micros))}`
    : `≈ ${sign}${formatReais(Math.abs(centavos))}`;
}
