import { expect, describe as group, test } from "bun:test";
import type { ModelChoice } from "@/api/chat";
import { colour, describe, position } from "./cost";

function model(id: string, cost: number | null): ModelChoice {
  return { id, label: id, vendor: "google", unavailable: null, cache_seconds: 3600, cost };
}

// The 2026-10-03 rates as the server sends them.
const models = [
  model("gemini-3.8-flash", 19),
  model("gemini-3.5-flash-lite", 9),
  model("claude-opus-5-5", 100),
  model("claude-sonnet-5-5", 50),
];

group("the cost bar", () => {
  test("runs from the cheapest to the dearest offered", () => {
    expect(position(9, models)).toBe(0);
    expect(position(100, models)).toBe(1);
    expect(position(50, models)).toBeCloseTo(41 / 91);
  });

  test("is green at the cheapest and red at the dearest", () => {
    expect(colour(0)).toBe("hsl(120 70% 45%)");
    expect(colour(1)).toBe("hsl(0 70% 45%)");
  });

  test("says in words where each model sits", () => {
    const said = models.map((choice) => describe(position(choice.cost ?? 0, models)));
    expect(said).toEqual(["inexpensive", "cheapest", "most expensive", "moderately expensive"]);
  });

  test("ignores an unpriced model and copes with a single priced one", () => {
    const one = [model("a", 40), model("b", null)];
    expect(position(40, one)).toBe(0);
    expect(describe(position(40, one))).toBe("cheapest");
  });
});
