// Reais typed and shown as integer centavos, never floated.

import { describe, expect, it } from "bun:test";
import { formatReais, parseReais, wholeCents, withinLimits } from "@/payments/reais";

describe("parseReais", () => {
  it("reads the Brazilian way and the English habit", () => {
    expect(parseReais("50")).toBe(5_000);
    expect(parseReais("50,5")).toBe(5_050);
    expect(parseReais("R$ 50,05")).toBe(5_005);
    expect(parseReais("1.000,50")).toBe(100_050);
    expect(parseReais("2.000")).toBe(200_000);
    expect(parseReais("50.5")).toBe(5_050);
    expect(parseReais("1000,00")).toBe(100_000);
    // Half-typed, it already reads as what it will be.
    expect(parseReais("50,")).toBe(5_000);
  });

  it("refuses what is not an amount", () => {
    for (const text of ["", "abc", ",5", "50,555", "1,2,3", "1.2.3", "-5", "5e3", "10.00,5"]) {
      expect([text, parseReais(text)]).toEqual([text, null]);
    }
  });
});

describe("formatReais", () => {
  it("groups thousands with dots and keeps two decimals", () => {
    expect(formatReais(2_000)).toBe("R$ 20,00");
    expect(formatReais(5_005)).toBe("R$ 50,05");
    expect(formatReais(200_000)).toBe("R$ 2.000,00");
  });

  it("matches the hand-rolled grouping it replaced, across every size of amount", () => {
    const before = (centavos: number) => {
      const whole = Math.floor(centavos / 100)
        .toString()
        .replace(/\B(?=(\d{3})+(?!\d))/g, ".");
      return `R$ ${whole},${String(centavos % 100).padStart(2, "0")}`;
    };
    for (const centavos of [0, 1, 99, 100, 99_999, 100_000, 100_050, 123_456_789, 500_000]) {
      expect([centavos, formatReais(centavos)]).toEqual([centavos, before(centavos)]);
    }
  });
});

it("knows the purchase limits and floors credits to the cent", () => {
  expect(withinLimits(1_999)).toBe(false);
  expect(withinLimits(2_000)).toBe(true);
  expect(withinLimits(500_000)).toBe(true);
  expect(withinLimits(500_001)).toBe(false);
  expect(wholeCents(10_020_040)).toBe(10_020_000);
});
