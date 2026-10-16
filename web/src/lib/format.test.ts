// Numbers in the page's language, and seconds as a clock reads them.

import { expect, it } from "bun:test";
import { formatClock, formatNumber } from "@/lib/format";

it("writes a number the reader's way, with the decimals asked for", () => {
  expect(formatNumber(1.5, { digits: 1 })).toBe("1.5");
  expect(formatNumber(1.5, { digits: 1 }, "pt-BR")).toBe("1,5");
  expect(formatNumber(2, { digits: 1 })).toBe("2.0");
  expect(formatNumber(1234.5, {}, "pt-BR")).toBe("1.234,5");
});

it("rounds a length before splitting it, so nothing reads 0:60", () => {
  expect(formatClock(59.6, "round")).toBe("1:00");
  expect(formatClock(59.6, "floor")).toBe("0:59");
});

it("mentions hours only once there are hours", () => {
  expect(formatClock(3700, "round")).toBe("1:01:40");
  expect(formatClock(3599, "floor")).toBe("59:59");
});

it("rounds tenths into the next second", () => {
  expect(formatClock(59.96, "tenths")).toBe("1:00.0");
  expect(formatClock(4.5, "tenths")).toBe("0:04.5");
  expect(formatClock(-2, "round")).toBe("0:00");
});
