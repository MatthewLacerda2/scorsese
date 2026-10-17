// The picker's cost bar (#706) says what it measures (#1220): the word "Cost",
// in the reader's language, beside the bar and inside one badge — on screen,
// not only to a screen reader. And only in the open list (#1444): the closed
// picker names the chosen model and shows no bar.

import { describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ModelChoice } from "@/api/chat";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { ModelPicker } from "./ModelPicker";

const choice = (id: string, label: string, cost: number): ModelChoice => ({
  id,
  label,
  vendor: "anthropic",
  unavailable: null,
  cache_seconds: 3600,
  cost,
});

const models = [
  choice("claude-sonnet-5-5", "Claude Sonnet 5.5", 20),
  choice("claude-opus-5-5", "Claude Opus 5.5", 40),
  choice("claude-fable-5-1", "Claude Fable 5.1", 100),
];

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  test("the closed picker names the model; the open list badges each bar", () => {
    const view = render(
      <QueryClientProvider client={createQueryClient()}>
        <I18nProvider initial={language}>
          <ModelPicker projectId={1} model="claude-opus-5-5" models={models} turns={[]} />
        </I18nProvider>
      </QueryClientProvider>,
    );
    const t = CATALOGUES[language].chat;
    const trigger = screen.getByRole("combobox", { name: t.model.label });
    expect(trigger.textContent).toBe("Claude Opus 5.5");
    expect(trigger.querySelector('[data-slot="badge"]')).toBeNull();

    fireEvent.keyDown(trigger, { key: "Enter" });
    const badges = [...document.querySelectorAll('[data-slot="badge"]')];
    expect(badges.map((badge) => badge.textContent)).toEqual(models.map(() => t.model.costLabel));
    const opus = badges[1];
    expect(opus?.getAttribute("aria-label")).toBe(t.model.cost(t.cost.inexpensive));
    const bar = opus?.lastElementChild?.firstElementChild as HTMLElement | null;
    expect(bar?.style.width).toBe("40%");
    view.unmount();
  });
});
