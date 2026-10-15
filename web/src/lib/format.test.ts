// Numbers in the page's language.

import { expect, it } from "bun:test";
import { formatNumber } from "@/lib/format";

it("writes a number the reader's way, with the decimals asked for", () => {
  expect(formatNumber(1.5, { digits: 1 })).toBe("1.5");
  expect(formatNumber(1.5, { digits: 1 }, "pt-BR")).toBe("1,5");
  expect(formatNumber(2, { digits: 1 })).toBe("2.0");
  expect(formatNumber(1234.5, {}, "pt-BR")).toBe("1.234,5");
});
