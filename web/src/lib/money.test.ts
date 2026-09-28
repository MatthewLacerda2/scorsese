// Money from integer micro-dollars: nothing rounded away, nothing floated.

import { expect, test } from "bun:test";
import { formatDollars, formatMoney, formatMovement, formatReais, toCentavos } from "./money";

// Intl puts a no-break space between "R$" and the number.
const nbsp = (text: string) => text.replaceAll(" ", " ");

test("dollars keep two decimals, and more when a cent would hide the amount", () => {
  expect(formatDollars(0)).toBe("$0.00");
  expect(formatDollars(1_060_000)).toBe("$1.06");
  expect(formatDollars(2_500_000)).toBe("$2.50");
  expect(formatDollars(4_200)).toBe("$0.0042");
  expect(formatDollars(1)).toBe("$0.000001");
  expect(formatDollars(1_234_567_890_000)).toBe("$1,234,567.89");
  expect(formatDollars(-2_500_000)).toBe("−$2.50");
});

test("reais are written the Brazilian way", () => {
  expect(nbsp(formatReais(123_456))).toBe("R$ 1.234,56");
  expect(nbsp(formatReais(-5))).toBe("−R$ 0,05");
});

test("a balance is ≈ reais when the server converted it, dollars when it could not", () => {
  expect(nbsp(formatMoney(2_000_000, 1_086))).toBe("≈ R$ 10,86");
  expect(formatMoney(2_000_000, null)).toBe("$2.00");
});

test("a movement says which way the money went", () => {
  expect(nbsp(formatMovement(-1_060_000, -576))).toBe("≈ −R$ 5,76");
  expect(formatMovement(10_000_000, null)).toBe("+$10.00");
  expect(formatMovement(0, 0).includes("0,00")).toBe(true);
});

test("centavos match the server's rounding, half away from zero", () => {
  // $1.06 at R$ 5.4321 = R$ 5.758026 → 576 centavos.
  expect(toCentavos(1_060_000, 54_321)).toBe(576);
  expect(toCentavos(-1_060_000, 54_321)).toBe(-576);
  // Exactly half a centavo: $0.001 at R$ 5.0000 = 0.5 centavo → 1.
  expect(toCentavos(1_000, 50_000)).toBe(1);
  expect(toCentavos(-1_000, 50_000)).toBe(-1);
  // $9M at R$ 5.4321: micros × rate is past a float's exact integers (2^53).
  expect(toCentavos(9_000_000_000_000, 54_321)).toBe(4_888_890_000);
});
