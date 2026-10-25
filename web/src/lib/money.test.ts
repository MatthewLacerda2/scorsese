// Money from integer micro-dollars: nothing rounded away, nothing floated.

import { expect, test } from "bun:test";
import { dollarAmount, formatDollars, formatMovement } from "./money";

test("dollars keep two decimals, and more when a cent would hide the amount", () => {
  expect(formatDollars(0)).toBe("$0.00");
  expect(formatDollars(1_060_000)).toBe("$1.06");
  expect(formatDollars(2_500_000)).toBe("$2.50");
  expect(formatDollars(4_200)).toBe("$0.0042");
  expect(formatDollars(1)).toBe("$0.000001");
  expect(formatDollars(1_234_567_890_000)).toBe("$1,234,567.89");
  expect(formatDollars(-2_500_000)).toBe("−$2.50");
});

// The same cases `scorsese_providers::prices::dollars` pins (#1287), so a
// balance reads alike in an assistant reply and on the page. Only the minus
// differs: the page sets `−`, a terminal and a log keep `-`.
test("thousands are grouped as the server groups them", () => {
  expect(formatDollars(999_000_000)).toBe("$999.00");
  expect(formatDollars(1_000_000_000)).toBe("$1,000.00");
  expect(formatDollars(1_234_000_000)).toBe("$1,234.00");
  expect(formatDollars(-1_234_000_000)).toBe("−$1,234.00");
});

test("a movement says which way the money went", () => {
  expect(formatMovement(-1_060_000)).toBe("−$1.06");
  expect(formatMovement(10_000_000)).toBe("+$10.00");
  expect(formatMovement(0)).toBe("$0.00");
});

test("an amount said in a sentence has no symbol, and no cents when it is whole", () => {
  expect(dollarAmount(4_000_000)).toBe("4");
  expect(dollarAmount(4_500_000)).toBe("4.50");
  expect(dollarAmount(1_234_000_000)).toBe("1,234");
});
