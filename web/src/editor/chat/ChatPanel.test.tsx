// Full mode (#1027), driven the way a person drives it: "Full" gives the chat
// the page and its opposite takes it back, the mode outlives a reload for its
// own project only, and a draft outlives the switch. Mounted in happy-dom
// (src/test/dom.ts) with the hook the editor uses, over a stubbed server.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Conversation } from "@/api/chat";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { ChatPanel } from "./ChatPanel";
import { useFull } from "./full";

const t = CATALOGUES.en.chat;

const conversation = (project: number): Conversation => ({
  project,
  session: null,
  turns: [],
  model: "m",
  models: [
    { id: "m", label: "M", vendor: "anthropic", unavailable: null, cache_seconds: 300, cost: null },
  ],
});

let previous: Parameters<typeof setFetch>[0];
const realEventSource = globalThis.EventSource;

beforeAll(() => {
  previous = setFetch(async (input) => {
    const project = Number(input.match(/projects\/(\d+)/)?.[1]);
    return new Response(JSON.stringify(conversation(project)));
  });
  // The live stream is never opened for real: a source that stays silent.
  globalThis.EventSource = class {
    close() {}
  } as unknown as typeof EventSource;
});

afterAll(() => {
  setFetch(previous);
  globalThis.EventSource = realEventSource;
});

/** The editor's wiring in miniature: the panel, told its mode by the same hook. */
function Editor({ project }: { project: number }) {
  const [full, setFull] = useFull(project);
  return (
    <div data-full={full}>
      <ChatPanel projectId={project} full={full} onFull={setFull} />
    </div>
  );
}

const open = (project: number) =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial="en">
        <Editor project={project} />
      </I18nProvider>
    </QueryClientProvider>,
  );

const mode = (view: ReturnType<typeof open>) =>
  view.container.querySelector("[data-full]")?.getAttribute("data-full");

test("Full gives the chat the page, and its opposite gives it back", async () => {
  const view = open(4);
  expect(mode(view)).toBe("false");
  await userEvent.click(screen.getByRole("button", { name: t.full.enter }));
  expect(mode(view)).toBe("true");
  await userEvent.click(screen.getByRole("button", { name: t.full.leave }));
  expect(mode(view)).toBe("false");
  expect(screen.getByRole("button", { name: t.full.enter })).toBeDefined();
});

test("the mode outlives a reload for its own project, and no other", async () => {
  const first = open(4);
  await userEvent.click(screen.getByRole("button", { name: t.full.enter }));
  first.unmount();
  const again = open(4);
  expect(mode(again)).toBe("true");
  again.unmount();
  expect(mode(open(5))).toBe("false");
});

test("a draft outlives the switch, both ways", async () => {
  open(4);
  const box = screen.getByPlaceholderText(t.composer.placeholder);
  await userEvent.type(box, "cut the intro");
  await userEvent.click(screen.getByRole("button", { name: t.full.enter }));
  expect(screen.getByPlaceholderText(t.composer.placeholder)).toBe(box);
  expect((box as HTMLTextAreaElement).value).toBe("cut the intro");
  await userEvent.click(screen.getByRole("button", { name: t.full.leave }));
  expect((box as HTMLTextAreaElement).value).toBe("cut the intro");
});
