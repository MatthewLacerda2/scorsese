// Reais as a Brazilian types and reads them, held as integer centavos — the
// unit the server charges in (docs/web.md, *Buying credits*). Never a float:
// "50,10" is 5010 centavos by string arithmetic, not 50.1 × 100.

/** The least one purchase may be, in centavos — `payments::MINIMUM_CENTAVOS`. */
export const MINIMUM_CENTAVOS = 2_000;

/** The most one purchase may be, in centavos — `payments::MAXIMUM_CENTAVOS`. */
export const MAXIMUM_CENTAVOS = 200_000;

/**
 * What a person typed as centavos, or `null` when it is not an amount.
 *
 * A comma is the decimal mark and dots group thousands (`1.000,50`), as in
 * Brazil. With no comma, a dot followed by exactly three digits groups
 * thousands (`2.000`) and any other dot is a decimal mark (`50.5`), so an
 * English habit still reads right. `R$` and spaces are ignored.
 */
export function parseReais(text: string): number | null {
  const bare = text.replace(/R\$/gi, "").replace(/\s/g, "");
  if (!bare) return null;
  let whole: string;
  let fraction = "";
  if (bare.includes(",")) {
    const parts = bare.split(",");
    if (parts.length !== 2) return null;
    [whole, fraction] = parts as [string, string];
    if (!/^\d{1,3}(\.\d{3})*$|^\d+$/.test(whole)) return null;
    whole = whole.replace(/\./g, "");
  } else if (/^\d{1,3}(\.\d{3})+$/.test(bare)) {
    whole = bare.replace(/\./g, "");
  } else {
    const parts = bare.split(".");
    if (parts.length > 2) return null;
    whole = parts[0] ?? "";
    fraction = parts[1] ?? "";
  }
  if (!/^\d+$/.test(whole) || !/^\d{0,2}$/.test(fraction)) return null;
  const centavos = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  return Number.isSafeInteger(centavos) ? centavos : null;
}

/** Centavos as reais, the Brazilian way: `R$ 1.000,50`. */
export function formatReais(centavos: number): string {
  const whole = Math.floor(centavos / 100)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `R$ ${whole},${String(centavos % 100).padStart(2, "0")}`;
}

/** Whether `centavos` is an amount one purchase may be. */
export function withinLimits(centavos: number): boolean {
  return centavos >= MINIMUM_CENTAVOS && centavos <= MAXIMUM_CENTAVOS;
}

/** Micro-dollars of credit, floored to the cent, for showing what reais buy. */
export function wholeCents(micros: number): number {
  return micros - (micros % 10_000);
}
