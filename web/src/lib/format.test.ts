// Numbers in the page's language, and seconds as a clock reads them.

import { expect, it } from "bun:test";
import { formatBytes, formatClock, formatNumber } from "@/lib/format";

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

// The same cases `scorsese_core::bytes` pins (#1287): decimal units, one
// decimal under 100, none from 100 up — on every surface alike.
it("says a size the way every surface says it", () => {
  expect(formatBytes(0)).toBe("0 B");
  expect(formatBytes(999)).toBe("999 B");
  expect(formatBytes(1_000)).toBe("1.0 KB");
  expect(formatBytes(1_536_000)).toBe("1.5 MB");
  expect(formatBytes(99_400_000)).toBe("99.4 MB");
  expect(formatBytes(250_000_000)).toBe("250 MB");
  expect(formatBytes(20_000_000_000)).toBe("20.0 GB");
  expect(formatBytes(3_200_000_000_000)).toBe("3.2 TB");
  expect(formatBytes(1_024)).toBe("1.0 KB");
  expect(formatBytes(1_048_576)).toBe("1.0 MB");
  expect(formatBytes(21_474_836_480)).toBe("21.5 GB");
});
