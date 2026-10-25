// The signed-in home: its sidebar lists the real projects and renames and
// deletes them in place.

import { afterEach, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Suspense } from "react";
import { MemoryRouter } from "react-router";
import type { ProjectSummary } from "@/api";
import { setFetch } from "@/api/client";
import { HomePage } from "@/app/pages";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";

const project = (id: number, name: string): ProjectSummary => ({
  id,
  name,
  revision: 1,
  created_at: 0,
  updated_at: 0,
  platform: null,
  style: null,
});

const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

/** Draws the page with `projects` seeded; returns the requests it sent. */
async function open() {
  const calls: string[] = [];
  setFetch(async (input, init) => {
    calls.push(`${init?.method ?? "GET"} ${input}`);
    return init?.method === "GET" || !init?.method
      ? Response.json([project(2, "Newer"), project(1, "Older")])
      : Response.json({ revision: 2 });
  });
  const client = createQueryClient();
  client.setQueryData(["projects"], [project(2, "Newer"), project(1, "Older")]);
  client.setQueryData(["credits", "balance"], { balance_micros: 5_000_000 });
  render(
    <I18nProvider initial="en">
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <Suspense fallback={null}>
            <HomePage />
          </Suspense>
        </MemoryRouter>
      </QueryClientProvider>
    </I18nProvider>,
  );
  await screen.findByRole("navigation", { name: "Main menu" });
  return calls;
}

test("the sidebar lists the projects, newest first, each opening the editor", async () => {
  await open();
  const links = screen
    .getAllByRole("link")
    .filter((link) => link.getAttribute("href")?.startsWith("/projects/"));
  expect(links.map((link) => link.textContent)).toEqual(["Newer", "Older"]);
  expect(links[0]?.getAttribute("href")).toBe("/projects/2/edit");
  expect(screen.getByRole("link", { name: "Library" }).getAttribute("href")).toBe("/library");
  expect(screen.getByRole("button", { name: "$5.00" })).toBeDefined();
  expect(screen.getByRole("link", { name: "Spending history" })).toBeDefined();
  expect(screen.getByRole("button", { name: "Log out" })).toBeDefined();
});

test("Rename turns the row into a field and Enter saves the new name", async () => {
  const calls = await open();
  await userEvent.click(screen.getByRole("button", { name: "Options for Older" }));
  await userEvent.click(await screen.findByRole("menuitem", { name: "Rename" }));
  const field = screen.getByRole("textbox", { name: "Project name" });
  await userEvent.clear(field);
  await userEvent.type(field, "Renamed{Enter}");
  expect(calls).toContain("PATCH /api/projects/1");
  expect(screen.queryByRole("textbox", { name: "Project name" })).toBeNull();
});

test("Escape, or leaving the name as it was, renames nothing", async () => {
  const calls = await open();
  await userEvent.click(screen.getByRole("button", { name: "Options for Older" }));
  await userEvent.click(await screen.findByRole("menuitem", { name: "Rename" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Project name" }), "x{Escape}");
  expect(screen.queryByRole("textbox", { name: "Project name" })).toBeNull();
  await userEvent.click(screen.getByRole("button", { name: "Options for Older" }));
  await userEvent.click(await screen.findByRole("menuitem", { name: "Rename" }));
  await userEvent.tab();
  expect(calls.filter((call) => call.startsWith("PATCH"))).toEqual([]);
});

test("Delete asks first, then deletes", async () => {
  const calls = await open();
  await userEvent.click(screen.getByRole("button", { name: "Options for Newer" }));
  await userEvent.click(await screen.findByRole("menuitem", { name: "Delete" }));
  expect(calls).not.toContain("DELETE /api/projects/2");
  await userEvent.click(
    within(await screen.findByRole("dialog")).getByRole("button", { name: "Delete" }),
  );
  expect(calls).toContain("DELETE /api/projects/2");
});
