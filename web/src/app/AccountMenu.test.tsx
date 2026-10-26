// The account menu names the person, with their email under it (#1266), and
// an account Google gave no name shows the email alone.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { Account } from "@/api";
import { createQueryClient, ME } from "@/app/queryClient";
import { AccountMenu } from "./AccountMenu";

async function opened(account: Account) {
  const client = createQueryClient();
  client.setQueryData(ME, account);
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <AccountMenu />
      </MemoryRouter>
    </QueryClientProvider>,
  );
  const trigger = screen.getByRole("button", { name: "Your account" });
  // Popper places the menu after the key, so the act waits for it too.
  await act(async () => {
    fireEvent.keyDown(trigger, { key: "Enter" });
  });
}

test("the menu shows the name, with the email under it", async () => {
  await opened({ id: 1, email: "ana@example.com", name: "Ana Souza", created_at: 0 });
  const name = screen.getByText("Ana Souza");
  const email = screen.getByText("ana@example.com");
  expect(name.parentElement).toBe(email.parentElement);
  expect(name.compareDocumentPosition(email) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

test("without a name, the email alone", async () => {
  await opened({ id: 1, email: "ana@example.com", created_at: 0 });
  const email = screen.getByText("ana@example.com");
  expect(email.parentElement?.textContent).toBe("ana@example.com");
});

test("the menu is the name, Settings and Log out", async () => {
  await opened({ id: 1, email: "ana@example.com", created_at: 0 });
  const items = screen.getAllByRole("menuitem").map((item) => item.textContent);
  expect(items).toEqual(["Settings", "Log out"]);
});

test("Settings opens the language and theme controls", async () => {
  await opened({ id: 1, email: "ana@example.com", created_at: 0 });
  await act(async () => {
    fireEvent.click(screen.getByRole("menuitem", { name: "Settings" }));
  });
  const dialog = await screen.findByRole("dialog");
  expect(within(dialog).getByRole("heading", { name: "Language" })).toBeTruthy();
  expect(within(dialog).getByRole("heading", { name: "Theme" })).toBeTruthy();
});

test("the page is clickable again once Settings closes", async () => {
  await opened({ id: 1, email: "ana@example.com", created_at: 0 });
  await act(async () => {
    fireEvent.click(screen.getByRole("menuitem", { name: "Settings" }));
  });
  await screen.findByRole("dialog");
  await act(async () => {
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
  });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.body.style.pointerEvents).not.toBe("none");
});
