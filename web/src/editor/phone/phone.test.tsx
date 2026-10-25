// The editor on a phone (#1570), driven the way a person drives it: the chat
// is the page with no way out of full mode, the clapperboard swaps it for the
// video and back with the draft intact, and the menu opens the sidebar's
// drawer. Mounted in happy-dom over a stubbed server, with `matchMedia`
// answering as a phone's window does.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { type EditorProject, SCHEMA_VERSION } from "@/api";
import type { Conversation } from "@/api/chat";
import { setFetch } from "@/api/client";
import { AddCreditsProvider } from "@/app/addCredits";
import { createQueryClient } from "@/app/queryClient";
import { UploadsProvider } from "@/files/uploads";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { EditorPage } from "../EditorPage";

const t = CATALOGUES.en;

const project: EditorProject = {
  id: 4,
  name: "teaser",
  revision: 1,
  created_at: 0,
  updated_at: 0,
  platform: null,
  document: {
    schema_version: SCHEMA_VERSION,
    name: "teaser",
    timeline_fps: { num: 30, den: 1 },
    tracks: [
      { id: "v1", kind: "video", clips: [{ id: "c1", asset: "a1", start: 0, duration: 60 }] },
    ],
  },
} as EditorProject;

const conversation: Conversation = {
  project: 4,
  session: null,
  turns: [],
  model: "m",
  models: [
    { id: "m", label: "M", vendor: "anthropic", unavailable: null, cache_seconds: 300, cost: null },
  ],
  effort: "high",
};

let previous: Parameters<typeof setFetch>[0];
const realEventSource = globalThis.EventSource;
const realMatchMedia = globalThis.matchMedia;

beforeAll(() => {
  previous = setFetch(async (input) => {
    if (/projects\/4$/.test(input)) return new Response(JSON.stringify(project));
    if (/projects\/4\/chat$/.test(input)) return new Response(JSON.stringify(conversation));
    return new Response(JSON.stringify({ error: "not here" }), { status: 404 });
  });
  globalThis.EventSource = class {
    close() {}
  } as unknown as typeof EventSource;
  globalThis.matchMedia = ((query: string) => ({
    matches: query.includes("max-width"),
    addEventListener() {},
    removeEventListener() {},
  })) as unknown as typeof matchMedia;
});

afterAll(() => {
  setFetch(previous);
  globalThis.EventSource = realEventSource;
  globalThis.matchMedia = realMatchMedia;
});

const open = () =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial="en">
        <UploadsProvider>
          <AddCreditsProvider>
            <MemoryRouter initialEntries={["/projects/4/edit"]}>
              <Routes>
                <Route path="/projects/:id/edit" element={<EditorPage />} />
              </Routes>
            </MemoryRouter>
          </AddCreditsProvider>
        </UploadsProvider>
      </I18nProvider>
    </QueryClientProvider>,
  );

test("the chat is the page, with no Full to leave and no assets panel", async () => {
  open();
  expect(await screen.findByPlaceholderText(t.chat.composer.placeholder)).toBeDefined();
  expect(screen.queryByRole("button", { name: t.chat.full.leave })).toBeNull();
  expect(screen.queryByRole("button", { name: t.chat.full.enter })).toBeNull();
  expect(screen.queryByRole("button", { name: t.editor.page.resizeChat })).toBeNull();
  expect(screen.queryByRole("button", { name: t.editor.page.render })).toBeNull();
});

test("the clapperboard shows the video and Render, and the chat comes back as it was", async () => {
  open();
  const box = (await screen.findByPlaceholderText(
    t.chat.composer.placeholder,
  )) as HTMLTextAreaElement;
  await userEvent.type(box, "cut the intro");
  await userEvent.click(screen.getByRole("button", { name: t.editor.page.showVideo }));
  expect(screen.getByRole("button", { name: t.editor.preview.play })).toBeDefined();
  expect(screen.getByRole("button", { name: t.editor.page.render })).toBeDefined();
  // Hidden, never unmounted.
  expect(box.isConnected).toBe(true);
  expect(box.closest("aside")?.className).toContain("invisible");
  await userEvent.click(screen.getByRole("button", { name: t.editor.page.showChat }));
  expect(screen.getByPlaceholderText(t.chat.composer.placeholder)).toBe(box);
  expect(box.value).toBe("cut the intro");
  expect(box.closest("aside")?.className).not.toContain("invisible");
  expect(screen.queryByRole("button", { name: t.editor.preview.play })).toBeNull();
});

test("the video view's timeline only scrubs: a clip is not taken hold of, a lane not removed", async () => {
  open();
  await userEvent.click(await screen.findByRole("button", { name: t.editor.page.showVideo }));
  const clip = screen.getByTitle(t.editor.timeline.clipTitle("c1"));
  await userEvent.pointer([{ keys: "[MouseLeft>]", target: clip }, { coords: { x: 80 } }]);
  expect(clip.className).not.toContain("cursor-grabbing");
  expect(clip.className).not.toContain("ring-2");
  await userEvent.pointer({ keys: "[/MouseLeft]" });
  const bin = screen.getByRole("button", { name: t.editor.timeline.removeTrack("v1") });
  await userEvent.click(bin);
  expect(screen.queryByRole("alertdialog")).toBeNull();
});

test("the menu opens the sidebar's drawer: home, library, credits, settings", async () => {
  open();
  await userEvent.click(await screen.findByRole("button", { name: t.home.menu }));
  const drawer = within(await screen.findByRole("navigation", { name: t.home.nav }));
  expect(drawer.getByRole("link", { name: "scorsese" }).getAttribute("href")).toBe("/");
  expect(drawer.getByRole("link", { name: t.common.nav.library })).toBeDefined();
  expect(drawer.getByRole("button", { name: t.common.settings.title })).toBeDefined();
});
