import { expect, describe as group, test } from "bun:test";
import type { ModelChoice } from "@/api/chat";
import { en } from "@/i18n/en";
import { colour, describe, position } from "./cost";

function model(id: string, cost: number | null): ModelChoice {
  return { id, label: id, vendor: "anthropic", unavailable: null, cache_seconds: 3600, cost };
}

// Four made-up models, so every step of the scale is reached; the two offered
// today are only the ends of it.
const models = [model("c", 19), model("d", 9), model("a", 100), model("b", 50)];

group("the cost bar", () => {
  test("runs from the cheapest to the dearest offered", () => {
    expect(position(9, models)).toBe(0);
    expect(position(100, models)).toBe(1);
    expect(position(50, models)).toBeCloseTo(41 / 91);
  });

  test("is green at the cheapest and red at the dearest", () => {
    expect(colour(0)).toBe("color-mix(in oklch, var(--warning) 0%, var(--success))");
    expect(colour(0.5)).toBe("color-mix(in oklch, var(--destructive) 0%, var(--warning))");
    expect(colour(1)).toBe("color-mix(in oklch, var(--destructive) 100%, var(--warning))");
  });

  test("says in words where each model sits", () => {
    const said = models.map((choice) => describe(position(choice.cost ?? 0, models), en.chat.cost));
    expect(said).toEqual(["inexpensive", "cheapest", "most expensive", "moderately expensive"]);
  });

  test("ignores an unpriced model and copes with a single priced one", () => {
    const one = [model("a", 40), model("b", null)];
    expect(position(40, one)).toBe(0);
    expect(describe(position(40, one), en.chat.cost)).toBe("cheapest");
  });
});
