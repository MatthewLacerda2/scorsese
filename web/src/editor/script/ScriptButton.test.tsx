// The Script button (#1445): it opens the project's script rendered, an empty
// state when there is none, and an edit saved through `script_write` on the
// revision it was read at.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { ScriptButton } from "./ScriptButton";

const state: { text: string | null; revision: number } = { text: null, revision: 2 };
const writes: unknown[] = [];
let previous: Parameters<typeof setFetch>[0];

beforeAll(() => {
  previous = setFetch(async (input, init) => {
    const url = String(input);
    const project = () => ({
      id: 3,
      revision: state.revision,
      document: state.text === null ? {} : { script: "script.md" },
    });
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    if (url.endsWith("/tools/script_read"))
      return Response.json({ said: [{ text: state.text, image: null }], project: project() });
    if (url.endsWith("/tools/script_write")) {
      writes.push(body);
      state.text = body.arguments.text;
      state.revision += 1;
      return Response.json({ said: [{ text: "written", image: null }], project: project() });
    }
    return Response.json(project());
  });
});

afterAll(() => setFetch(previous));

function shown(language: (typeof LANGUAGES)[number]["language"]) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial={language}>
        <ScriptButton projectId={3} labelled />
      </I18nProvider>
    </QueryClientProvider>,
  );
}

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  const t = CATALOGUES[language].chat.script;

  test("no script: the empty state, and one can be written", async () => {
    Object.assign(state, { text: null, revision: 2 });
    writes.length = 0;
    const view = shown(language);
    fireEvent.click(screen.getByRole("button", { name: t.open }));
    expect(await screen.findByText(t.empty)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: t.write }));
    fireEvent.change(screen.getByRole("textbox", { name: t.title }), {
      target: { value: "# Scene one" },
    });
    fireEvent.click(screen.getByRole("button", { name: t.save }));
    await waitFor(() => expect(writes).toHaveLength(1));
    expect(writes[0]).toEqual({ arguments: { text: "# Scene one" }, revision: 2 });
    expect(await screen.findByText("Scene one")).toBeTruthy();
    view.unmount();
  });

  test("a script is shown rendered; Cancel leaves it as it was", async () => {
    state.text = "A **door** opens.";
    writes.length = 0;
    const view = shown(language);
    fireEvent.click(screen.getByRole("button", { name: t.open }));
    const bold = await screen.findByText("door");
    expect(bold.tagName).toBe("STRONG");
    fireEvent.click(screen.getByRole("button", { name: t.edit }));
    const box = screen.getByRole("textbox", { name: t.title }) as HTMLTextAreaElement;
    expect(box.value).toBe("A **door** opens.");
    fireEvent.click(screen.getByRole("button", { name: t.cancel }));
    expect((await screen.findByText("door")).tagName).toBe("STRONG");
    expect(writes).toHaveLength(0);
    view.unmount();
  });
});
