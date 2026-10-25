// `/start`, where a prompt written signed out arrives after the sign-in
// (#1535): it makes the project and sends the prompt; a reload makes nothing;
// no credit leaves the prompt in the new project's chat box with the reason;
// and a file that did not come through is named there to attach again.

import { afterEach, expect, test } from "bun:test";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";
import { browserStorage, KEYS } from "@/lib/storage";
import { keep } from "@/prompt/handOff";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };
const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

const made = {
  id: 7,
  name: "x",
  revision: 1,
  created_at: 0,
  updated_at: 0,
  document: { name: "x", timeline_fps: { num: 30, den: 1 }, tracks: [], assets: [] },
};
const conversation = {
  project: 7,
  session: null,
  turns: [],
  model: "m",
  models: [
    { id: "m", label: "M", vendor: "anthropic", unavailable: null, cache_seconds: 300, cost: null },
  ],
  effort: "high",
};

/** `/start`, signed in; `chat` answers the first message. Returns each write made. */
function arrive(chat: () => Response) {
  const writes: { call: string; body: unknown }[] = [];
  setFetch(async (input, init) => {
    const call = `${init?.method ?? "GET"} ${input}`;
    const body = typeof init?.body === "string" ? JSON.parse(init.body) : null;
    if (init?.method === "POST") writes.push({ call, body });
    if (call === "POST /api/projects") return Response.json(made);
    if (call === "POST /api/projects/7/chat") return chat();
    if (call === "GET /api/projects/7") return Response.json(made);
    if (call === "GET /api/projects/7/chat") return Response.json(conversation);
    return Response.json([]);
  });
  const client = createQueryClient();
  client.setQueryData(ME, ana);
  client.setQueryData(["projects"], []);
  client.setQueryData(["credits", "balance"], { balance_micros: 0 });
  render(
    <App
      language="en"
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={["/start"]}>{routes}</MemoryRouter>}
    />,
  );
  return writes;
}

const sent = () => Response.json({ id: 1 }, { status: 202 });

test("the kept prompt makes a named project and is its first message, and is taken once", async () => {
  await keep("quero um vídeo de lançamento da minha bicicleta elétrica", []);
  const writes = arrive(sent);
  await waitFor(() =>
    expect(writes.map((write) => write.call)).toContain("POST /api/projects/7/chat"),
  );
  expect(writes[0]).toEqual({
    call: "POST /api/projects",
    body: { name: "Lançamento Bicicleta Elétrica" },
  });
  expect(writes[1]?.body).toEqual({
    prompt: "quero um vídeo de lançamento da minha bicicleta elétrica",
    fresh: false,
    attachments: [],
  });
  expect(browserStorage()?.getItem(KEYS.handOff)).toBeNull();
});

test("arriving with nothing kept, as a reload does, makes nothing and goes home", async () => {
  const writes = arrive(sent);
  expect(await screen.findByRole("textbox", { name: "Describe your video" })).toBeDefined();
  expect(writes).toEqual([]);
});

test("no credit at all: the project is made and opened, the prompt waits in its chat box", async () => {
  await keep("a trailer for my band", []);
  arrive(() => Response.json({ error: "You have no credit left." }, { status: 402 }));
  expect(await screen.findByText("You have no credit left for the assistant.")).toBeDefined();
  expect(screen.getByDisplayValue("a trailer for my band")).toBeDefined();
});

test("a file that did not come through is named, and the prompt waits unsent", async () => {
  // This test's browser has no IndexedDB: the file's name is kept, the file is not.
  await keep("a slideshow", [new File(["png"], "still.png", { type: "image/png" })]);
  const writes = arrive(sent);
  expect(await screen.findByText("still.png")).toBeDefined();
  expect(screen.getByDisplayValue("a slideshow")).toBeDefined();
  expect(writes.map((write) => write.call)).toEqual(["POST /api/projects"]);
});
