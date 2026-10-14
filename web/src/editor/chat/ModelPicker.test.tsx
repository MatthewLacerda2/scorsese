// The picker's cost bar (#706) says what it measures (#1220): the word "Cost",
// in the reader's language, beside the bar and inside one badge — on screen,
// not only to a screen reader.

import { describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ModelChoice } from "@/api/chat";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { ModelPicker } from "./ModelPicker";

const choice = (id: string, cost: number): ModelChoice => ({
  id,
  label: id,
  vendor: "anthropic",
  unavailable: null,
  cache_seconds: 3600,
  cost,
});

const models = [choice("claude-sonnet-5-5", 50), choice("claude-opus-5-5", 100)];

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  test("the chosen model's bar sits in a badge that says what it is", () => {
    const view = render(
      <QueryClientProvider client={createQueryClient()}>
        <I18nProvider initial={language}>
          <ModelPicker projectId={1} model="claude-sonnet-5-5" models={models} turns={[]} />
        </I18nProvider>
      </QueryClientProvider>,
    );
    const t = CATALOGUES[language].chat;
    const badge = view.container.querySelector('[data-slot="badge"]');
    expect(badge?.textContent).toBe(t.model.costLabel);
    expect(badge?.getAttribute("aria-label")).toBe(t.model.cost(t.cost.cheapest));
    const bar = badge?.lastElementChild?.firstElementChild as HTMLElement | null;
    expect(bar?.style.width).toBe("50%");
    view.unmount();
  });
});
