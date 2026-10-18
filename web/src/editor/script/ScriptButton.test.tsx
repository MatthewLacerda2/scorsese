// The Script button (#1445): it opens the project's script rendered, an empty
// state when there is none, and an edit saved through `script_write` on the
// revision it was read at — refused, with the person's version kept on screen,
// when the assistant rewrote the script meanwhile (#1488).

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { ScriptButton } from "./ScriptButton";

const state: { text: string | null; revision: number } = { text: null, revision: 2 };
const writes: unknown[] = [];
let previous: Parameters<typeof setFetch>[0];
const realEventSource = globalThis.EventSource;
// The live stream's one connection, so a test can say the project moved.
let stream: { onmessage?: (message: { data: string }) => void } = {};

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
      if (body.revision !== state.revision)
        return Response.json({ error: "the project moved on" }, { status: 409 });
      writes.push(body);
      state.text = body.arguments.text;
      state.revision += 1;
      return Response.json({ said: [{ text: "written", image: null }], project: project() });
    }
    return Response.json(project());
  });
  globalThis.EventSource = class {
    onmessage?: (message: { data: string }) => void;
    constructor() {
      stream = this;
    }
    close() {}
  } as unknown as typeof EventSource;
});

afterAll(() => {
  setFetch(previous);
  globalThis.EventSource = realEventSource;
});

/** The assistant writes `text` as the script, and the stream says so. */
function rewritten(text: string) {
  Object.assign(state, { text, revision: state.revision + 1 });
  const event = { type: "project", id: 3, revision: state.revision };
  act(() => stream.onmessage?.({ data: JSON.stringify(event) }));
}

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

describe("the assistant rewrote it meanwhile", () => {
  const t = CATALOGUES.en.chat.script;

  test("the save is refused, the script reloaded, and the person's version kept", async () => {
    Object.assign(state, { text: "A door opens.", revision: 2 });
    writes.length = 0;
    const view = shown("en");
    fireEvent.click(screen.getByRole("button", { name: t.open }));
    await screen.findByText("A door opens.");
    fireEvent.click(screen.getByRole("button", { name: t.edit }));
    rewritten("A window shuts.");
    expect((await screen.findByRole("status")).textContent).toBe(t.changed);
    fireEvent.change(screen.getByRole("textbox", { name: t.title }), {
      target: { value: "A door slams." },
    });
    fireEvent.click(screen.getByRole("button", { name: t.save }));
    expect((await screen.findByRole("status")).textContent).toBe(t.moved);
    expect(await screen.findByText("A window shuts.")).toBeTruthy();
    const kept = screen.getByRole("textbox", { name: t.yours }) as HTMLTextAreaElement;
    expect(kept.value).toBe("A door slams.");
    expect(kept.readOnly).toBe(true);
    expect(writes).toHaveLength(0);
    expect(state.text).toBe("A window shuts.");
    view.unmount();
  });

  test("an open script follows the assistant's rewrite", async () => {
    Object.assign(state, { text: "A door opens.", revision: 2 });
    const view = shown("en");
    fireEvent.click(screen.getByRole("button", { name: t.open }));
    await screen.findByText("A door opens.");
    rewritten("A window shuts.");
    expect(await screen.findByText("A window shuts.")).toBeTruthy();
    view.unmount();
  });
});
