// The editor's arithmetic for a project's shape, held to the server's: a
// render that names no size and the one the Render dialog offers first agree.

import { expect, test } from "bun:test";
import { aspectOf, aspectOfSize, deliveries } from "./shape";

test("a document that names no shape is 16:9", () => {
  expect(aspectOf({})).toBe("16:9");
  expect(aspectOf({ aspect: "9:16" })).toBe("9:16");
});

test("the first size is Ratio::delivery's: 1080 lines on the shorter side", () => {
  expect(deliveries("16:9")).toEqual(["1920x1080", "1280x720"]);
  expect(deliveries("9:16")).toEqual(["1080x1920", "720x1280"]);
  expect(deliveries("1:1")).toEqual(["1080x1080", "720x720"]);
  expect(deliveries("4:5")).toEqual(["1080x1350", "720x900"]);
  expect(deliveries("21:9")).toEqual(["2520x1080", "1680x720"]);
});

test("a size's shape is in lowest terms", () => {
  expect(aspectOfSize(1080, 1920)).toBe("9:16");
  expect(aspectOfSize(1080, 1080)).toBe("1:1");
  expect(aspectOfSize(1080, 1350)).toBe("4:5");
});
