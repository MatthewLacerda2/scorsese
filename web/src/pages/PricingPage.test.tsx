// The pricing page at /pricing: public, the cards with their bonuses, and
// what each button does signed out (the sign-in popup) and signed in (home, or
// Add credits with the card's amount).

import { afterEach, expect, test } from "bun:test";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };
const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

function visit(account: Account | null) {
  setFetch(async () => Response.json({ error: "no quote in this test" }, { status: 500 }));
  const client = createQueryClient();
  client.setQueryData(ME, account);
  client.setQueryData(["projects"], []);
  client.setQueryData(["credits", "balance"], { balance_micros: 0 });
  render(
    <App
      language="en"
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={["/pricing"]}>{routes}</MemoryRouter>}
    />,
  );
}

test("the page is public and shows the free card and the three purchase sizes with their bonuses", async () => {
  visit(null);
  expect(
    await screen.findByRole("heading", { level: 1, name: "Pay for what you make" }),
  ).toBeDefined();
  expect(screen.getByText("Free")).toBeDefined();
  expect(screen.getByRole("heading", { name: "$4.00 a day" })).toBeDefined();
  for (const reais of ["R$ 20", "R$ 40", "R$ 80"]) {
    expect(screen.getByRole("heading", { name: reais })).toBeDefined();
  }
  expect(screen.getByText("+5% bonus credit")).toBeDefined();
  expect(screen.getByText("+10% bonus credit")).toBeDefined();
  // The R$ 80 card is the highlighted one.
  expect(screen.getByText("Best value").closest("[data-best]")?.getAttribute("data-best")).toBe(
    "true",
  );
  expect(document.body.textContent).not.toMatch(/subscription/i);
});

test("it lists what things cost and answers the questions", async () => {
  visit(null);
  await screen.findByRole("heading", { name: "What things cost" });
  expect(screen.getByText("≈ $0.11")).toBeDefined();
  expect(screen.getByText("≈ $0.02–$0.04")).toBeDefined();
  expect(screen.getByText("≈ $1.44")).toBeDefined();
  expect(screen.getByText(/usually fits in a day's free credit/)).toBeDefined();
  expect(screen.getByText("Do credits expire?")).toBeDefined();
  expect(screen.getByRole("link", { name: "Read the terms" }).getAttribute("href")).toBe("/terms");
});

test("signed out, the buttons open the sign-in popup", async () => {
  visit(null);
  await userEvent.click(await screen.findByRole("button", { name: "Start free" }));
  expect(await screen.findByRole("dialog")).toBeDefined();
});

test("signed out, a buy button opens the sign-in popup too", async () => {
  visit(null);
  const buttons = await screen.findAllByRole("button", { name: "Buy" });
  await userEvent.click(buttons[1] as HTMLElement);
  expect(within(await screen.findByRole("dialog")).getByText("Welcome to scorsese")).toBeDefined();
});

test("signed in, a buy button opens Add credits with that card's amount", async () => {
  visit(ana);
  const buttons = await screen.findAllByRole("button", { name: "Buy" });
  await userEvent.click(buttons[1] as HTMLElement);
  const amount = await screen.findByLabelText("Amount in reais");
  expect((amount as HTMLInputElement).value).toBe("40");
});
