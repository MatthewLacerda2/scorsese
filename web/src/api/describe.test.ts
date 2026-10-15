import { describe, expect, test } from "bun:test";
import { ApiError } from "./client";
import { describeError } from "./describe";

describe("describeError", () => {
  const conflict = new ApiError(409, "the server's words");

  test("a refusal keeps the server's words without a better set", () => {
    expect(describeError(conflict)).toBe("the server's words");
  });

  test("a status the caller has words for gets them", () => {
    expect(describeError(conflict, { 409: "busy" })).toBe("busy");
    expect(describeError(new ApiError(500, "boom"), { 409: "busy" })).toBe("boom");
  });

  test("anything else shows the fallback, or its own message", () => {
    expect(describeError(new Error("nope"), { 409: "busy" }, "failed")).toBe("failed");
    expect(describeError(new Error("nope"))).toBe("nope");
    expect(describeError("bare")).toBe("bare");
  });
});
