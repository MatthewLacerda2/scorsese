// The effort picker (#1443) in the chat panel's top bar, beside the model: it
// shows the project's level as the server keeps it, in the reader's words, and
// a pick is written to the project — never sent with a message.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { EffortPicker } from "./EffortPicker";

const sent: { method: string; url: string; body: unknown }[] = [];
let previous: Parameters<typeof setFetch>[0];

beforeAll(() => {
  previous = setFetch(async (input, init) => {
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    sent.push({ method: init?.method ?? "GET", url: String(input), body });
    return new Response(JSON.stringify(body ?? {}));
  });
});

afterAll(() => setFetch(previous));

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  test("the closed picker names the project's level; a pick is kept on the project", async () => {
    sent.length = 0;
    const view = render(
      <QueryClientProvider client={createQueryClient()}>
        <I18nProvider initial={language}>
          <EffortPicker projectId={3} effort="xhigh" />
        </I18nProvider>
      </QueryClientProvider>,
    );
    const t = CATALOGUES[language].chat.effort;
    const trigger = screen.getByRole("combobox", { name: t.label });
    expect(trigger.textContent).toBe(t.name.xhigh);
    expect(trigger.getAttribute("title")).toBe(t.hint.xhigh);

    fireEvent.keyDown(trigger, { key: "Enter" });
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual([
      t.name.low,
      t.name.medium,
      t.name.high,
      t.name.xhigh,
    ]);
    fireEvent.click(options[0] as HTMLElement);
    await waitFor(() => expect(sent.some((call) => call.method === "PUT")).toBe(true));
    expect(sent.find((call) => call.method === "PUT")).toMatchObject({
      url: expect.stringContaining("/projects/3/chat/effort"),
      body: { effort: "low" },
    });
    view.unmount();
  });
});
