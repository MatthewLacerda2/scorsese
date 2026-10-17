// "New conversation" (#1437) is not offered while a turn runs: starting afresh
// would empty the view and with it the Stop button, and the next message would
// open a second session beside the turn still spending. Mounted in happy-dom
// over a stubbed server, as `ChatPanel.test.tsx` is.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { Conversation, TurnState, TurnView } from "@/api/chat";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { ChatPanel } from "./ChatPanel";

const t = CATALOGUES.en.chat;

const turn = (project: number, state: TurnState): TurnView => ({
  id: project,
  session: 1,
  project,
  prompt: "make it shorter",
  state,
  answer: state === "running" ? null : "Done.",
  stop_reason: null,
  model: "m",
  calls: 0,
  input_tokens: 0,
  output_tokens: 0,
  cache_write_tokens: 0,
  cache_read_tokens: 0,
  charged_micros: 0,
  quote: null,
  quote_answer: null,
  questions: [],
  started_at: 0,
  finished_at: state === "running" ? null : 1,
});

// Project 1's only turn is still running; project 2's has answered.
const conversation = (project: number): Conversation => ({
  project,
  session: 1,
  turns: [turn(project, project === 1 ? "running" : "answered")],
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
  globalThis.EventSource = class {
    close() {}
  } as unknown as typeof EventSource;
});

afterAll(() => {
  setFetch(previous);
  globalThis.EventSource = realEventSource;
});

const open = (project: number) =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial="en">
        <ChatPanel projectId={project} full={false} onFull={() => {}} />
      </I18nProvider>
    </QueryClientProvider>,
  );

test("a finished conversation offers a new one", async () => {
  const view = open(2);
  expect(await screen.findByRole("button", { name: t.composer.fresh })).toBeTruthy();
  view.unmount();
});

test("a running turn hides the offer, so its Stop stays in reach", async () => {
  const view = open(1);
  expect(await screen.findByRole("button", { name: t.composer.stop })).toBeTruthy();
  expect(screen.queryByRole("button", { name: t.composer.fresh })).toBeNull();
  view.unmount();
});
