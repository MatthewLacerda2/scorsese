// The pricing page at /pricing: public, its figures the server's, the cards
// with their bonuses and dollars, and what each button does signed out (the
// sign-in popup) and signed in (home, or Stripe's Checkout at the card's rate).

import { afterEach, expect, spyOn, test } from "bun:test";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account, PaymentQuote, Prices } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };
const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

function quote(centavos: number, credit: number, bonus: number, rate = "4.99"): PaymentQuote {
  return {
    brl_centavos: centavos,
    credit_micros: credit,
    bonus_micros: bonus,
    bonus_percent: centavos >= 8_000 ? 10 : centavos >= 4_000 ? 5 : 0,
    brl_per_usd: rate,
    rate_source: "awesomeapi",
    minimum_centavos: 2_000,
    maximum_centavos: 500_000,
  };
}

const PRICES: Prices = {
  examples: [
    { id: "image", from_micros: 110_000 },
    { id: "narration", from_micros: 20_000, to_micros: 40_000 },
    { id: "music", from_micros: 180_000 },
    { id: "video", from_micros: 1_440_000 },
  ],
  free: { daily_micros: 4_000_000, monthly_micros: 12_000_000 },
  recharge: {
    minimum_centavos: 2_000,
    maximum_centavos: 500_000,
    tiers: [
      { from_centavos: 4_000, percent: 5 },
      { from_centavos: 8_000, percent: 10 },
    ],
    cards: [
      { brl_centavos: 2_000, bonus_percent: 0, best: false, quote: quote(2_000, 4_008_016, 0) },
      {
        brl_centavos: 4_000,
        bonus_percent: 5,
        best: false,
        quote: quote(4_000, 8_016_032, 400_801),
      },
      { brl_centavos: 8_000, bonus_percent: 10, best: true, quote: null },
    ],
  },
};

/** What the page sent to the server, and what it answers a purchase with. */
let sent: { url: string; body: unknown }[] = [];

function visit(account: Account | null, purchase?: () => Response) {
  sent = [];
  setFetch(async (input, init) => {
    sent.push({ url: String(input), body: init?.body ? JSON.parse(String(init.body)) : null });
    if (String(input).endsWith("/api/payments") && purchase) return purchase();
    return Response.json({ error: "nothing else in this test" }, { status: 500 });
  });
  const client = createQueryClient();
  client.setQueryData(["prices"], PRICES);
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
  // The dollars each card buys at the day's rate, bonus included.
  expect(screen.getByText("≈ $4.00 in credits")).toBeDefined();
  expect(screen.getByText("≈ $8.41 in credits")).toBeDefined();
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

test("signed in, a buy button goes straight to Stripe at the rate its card shows", async () => {
  const assign = spyOn(window.location, "assign").mockImplementation(() => {});
  visit(ana, () =>
    Response.json({ url: "https://checkout.stripe.com/c/pay/1", session: "cs_1", quote: {} }),
  );
  const buttons = await screen.findAllByRole("button", { name: "Buy" });
  await userEvent.click(buttons[1] as HTMLElement);
  await waitFor(() => expect(assign).toHaveBeenCalledWith("https://checkout.stripe.com/c/pay/1"));
  const purchase = sent.find((call) => call.url.endsWith("/api/payments"));
  expect(purchase?.body).toEqual({ centavos: 4_000, brl_per_usd: "4.99" });
  assign.mockRestore();
});

test("signed in, a rate that moved shows the card's new figure and why", async () => {
  const moved = quote(4_000, 7_984_031, 399_201, "5.01");
  visit(ana, () =>
    Response.json(
      {
        error: "the dollar's rate just changed: check what your reais buy now and pay again",
        quote: moved,
      },
      { status: 409 },
    ),
  );
  const buttons = await screen.findAllByRole("button", { name: "Buy" });
  await userEvent.click(buttons[1] as HTMLElement);
  expect(await screen.findByRole("alert")).toHaveProperty(
    "textContent",
    "the dollar's rate just changed: check what your reais buy now and pay again",
  );
  expect(screen.getByText("≈ $8.38 in credits")).toBeDefined();
});

test("signed in, a card with no rate to show opens Add credits with its amount", async () => {
  visit(ana);
  const buttons = await screen.findAllByRole("button", { name: "Buy" });
  await userEvent.click(buttons[2] as HTMLElement);
  const amount = await screen.findByLabelText("Amount in reais");
  expect((amount as HTMLInputElement).value).toBe("80");
});
