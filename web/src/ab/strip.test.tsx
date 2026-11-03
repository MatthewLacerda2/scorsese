// The `hero-strip` test (#1555), waiting its turn: version `a`, today's page,
// has the film strip on the first screen; `b` keeps the first screen to the
// prompt box alone and starts the strip below it.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { draw } from "@/ab/draw";
import { TESTS } from "@/ab/tests";
import type { Prices } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { queryKeys } from "@/app/queryKeys";
import { I18nProvider } from "@/i18n/I18nProvider";
import { Hero } from "@/landing/Hero";

const PRICES: Prices = {
  examples: [],
  free: { daily_micros: 4_000_000, monthly_micros: 12_000_000 },
  recharge: null,
};

function firstScreen(layout?: string) {
  // What the free line and the box's "+" ask for, so mounting asks nothing.
  const client = createQueryClient();
  client.setQueryData(queryKeys.prices, PRICES);
  client.setQueryData(["formats"], { video: [], image: [], audio: [], html: [], text: [] });
  const { container } = render(
    <QueryClientProvider client={client}>
      <I18nProvider initial="en">
        <Hero onPrompt={async () => {}} layout={layout} />
      </I18nProvider>
    </QueryClientProvider>,
  );
  const screen = container.querySelector("[data-first-screen]");
  return {
    screen,
    holdsBox: screen?.querySelector("textarea") != null,
    holdsStrip: screen?.querySelector(".carousel") != null,
    strips: container.querySelectorAll(".carousel").length,
  };
}

test("the strip test is defined, waiting its turn, and draws both versions", () => {
  const strip = TESTS.find((t) => t.id === "hero-strip");
  expect(strip).toEqual({ id: "hero-strip", versions: ["a", "b"], running: false });
  const seen = new Set(
    Array.from({ length: 40 }, (_, i) => (strip ? draw(`visitor-${i}`, strip) : "")),
  );
  expect([...seen].sort()).toEqual(["a", "b"]);
});

test("version a, and no version, has the strip on the first screen", () => {
  for (const layout of ["a", undefined]) {
    const shown = firstScreen(layout);
    expect(shown.screen?.className).toContain("min-h-svh");
    expect(shown.holdsBox).toBe(true);
    expect(shown.holdsStrip).toBe(true);
  }
});

test("version b keeps the prompt box alone on the first screen, the strip below it", () => {
  const shown = firstScreen("b");
  expect(shown.screen?.className).toContain("min-h-svh");
  expect(shown.holdsBox).toBe(true);
  expect(shown.holdsStrip).toBe(false);
  expect(shown.strips).toBe(1);
});
