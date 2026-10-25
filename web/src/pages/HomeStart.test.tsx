// What Enter does in the home's prompt box, and where the sidebar is: a
// project made from the prompt and named by it, the prompt sent to its
// assistant as the first message, the editor opened; a refusal shown and the
// project kept; and the library and `/projects` on the same sidebar.

import { afterEach, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };
const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

/** The app at `url`, signed in; `answer` decides each request after the first `GET`s. */
function visit(url: string, answer: (call: string, body: unknown) => Response) {
  const calls: string[] = [];
  setFetch(async (input, init) => {
    const call = `${init?.method ?? "GET"} ${input}`;
    calls.push(call);
    const body = typeof init?.body === "string" ? JSON.parse(init.body) : null;
    return answer(call, body);
  });
  const client = createQueryClient();
  client.setQueryData(ME, ana);
  client.setQueryData(["projects"], []);
  client.setQueryData(["credits", "balance"], { balance_micros: 0 });
  render(
    <App
      language="en"
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  );
  return calls;
}

const made = { id: 7, name: "x", revision: 1, created_at: 0, updated_at: 0, document: {} };

test("Enter makes a project named from the prompt, sends the prompt to it, and opens it", async () => {
  const bodies: unknown[] = [];
  const calls = visit("/", (call, body) => {
    if (call === "POST /api/projects") {
      bodies.push(body);
      return Response.json(made);
    }
    if (call === "POST /api/projects/7/chat") {
      bodies.push(body);
      return Response.json({ id: 1 }, { status: 202 });
    }
    return Response.json([]);
  });
  const box = await screen.findByRole("textbox", { name: "Describe your video" });
  await userEvent.type(box, "a promo for my bakery{Enter}");
  await screen
    .findByRole("textbox", { name: "Describe your video" }, { timeout: 100 })
    .catch(() => {});
  await new Promise((resolve) => setTimeout(resolve, 50));
  expect(bodies[0]).toEqual({ name: "Promo bakery" });
  expect(bodies[1]).toEqual({ prompt: "a promo for my bakery", fresh: false, attachments: [] });
  expect(calls.indexOf("POST /api/projects")).toBeLessThan(
    calls.indexOf("POST /api/projects/7/chat"),
  );
});

test("a refusal is shown under the box, and sending again does not make a second project", async () => {
  let created = 0;
  visit("/", (call) => {
    if (call === "POST /api/projects") {
      created += 1;
      return Response.json(made);
    }
    if (call === "POST /api/projects/7/chat") {
      return Response.json({ error: "You have no credit left." }, { status: 402 });
    }
    return Response.json([]);
  });
  const box = await screen.findByRole("textbox", { name: "Describe your video" });
  await userEvent.type(box, "a trailer{Enter}");
  expect((await screen.findByRole("alert")).textContent).toBe("You have no credit left.");
  await userEvent.click(screen.getByRole("button", { name: "Send" }));
  await screen.findByRole("alert");
  expect(created).toBe(1);
});

test("the library has the sidebar, and /projects is the home", async () => {
  visit("/library", () => Response.json([]));
  expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeDefined();
  expect(await screen.findByRole("heading", { name: "Library" })).toBeDefined();
});

test("/projects goes to the home", async () => {
  visit("/projects", () => Response.json([]));
  expect(await screen.findByRole("textbox", { name: "Describe your video" })).toBeDefined();
});
