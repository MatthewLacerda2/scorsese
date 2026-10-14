// "Add credits" (#548): what the reais buy is shown before paying, and Pay is
// offered only for an amount the server will take.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import type { PaymentQuote } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { AddCreditsDialog } from "./AddCredits";

const FIFTY: PaymentQuote = {
  brl_centavos: 5_000,
  credit_micros: 10_020_040,
  brl_per_usd: "4.99",
  rate_source: "awesomeapi",
  minimum_centavos: 2_000,
  maximum_centavos: 500_000,
};

function dialog() {
  const client = createQueryClient();
  client.setQueryData(["payments", "quote", 5_000], FIFTY);
  render(
    <QueryClientProvider client={client}>
      <AddCreditsDialog open onOpenChange={() => {}} />
    </QueryClientProvider>,
  );
}

test("the credits the reais buy, and the rate, are shown before paying", () => {
  dialog();
  expect(screen.getByText("R$ 50,00 → $10.02 in credits")).toBeTruthy();
  expect(screen.getByText(/At R\$ 4.99 to the dollar/)).toBeTruthy();
  const pay = screen.getByRole("button", { name: "Pay" }) as HTMLButtonElement;
  expect(pay.disabled).toBe(false);
});

test("an amount that is not one, or is outside the limits, cannot be paid", () => {
  dialog();
  const field = screen.getByLabelText("Amount in reais");
  for (const value of ["abc", "10", "5.000,01"]) {
    fireEvent.change(field, { target: { value } });
    expect(screen.queryByText(/in credits/)).toBeNull();
    const pay = screen.getByRole("button", { name: "Pay" }) as HTMLButtonElement;
    expect(pay.disabled).toBe(true);
  }
});

test("Pay names the rate shown, and a rate that moved is shown before paying again", async () => {
  const sent: unknown[] = [];
  const moved = { ...FIFTY, credit_micros: 9_980_039, brl_per_usd: "5.01" };
  const previous = setFetch(async (_, init) => {
    sent.push(JSON.parse(String(init?.body)));
    const body = { error: "the dollar's rate just changed", quote: moved };
    return new Response(JSON.stringify(body), { status: 409 });
  });
  try {
    dialog();
    fireEvent.click(screen.getByRole("button", { name: "Pay" }));
    expect(await screen.findByText("R$ 50,00 → $9.98 in credits")).toBeTruthy();
    expect(sent).toEqual([{ centavos: 5_000, brl_per_usd: "4.99" }]);
    expect(screen.getByText(/rate just changed/)).toBeTruthy();
  } finally {
    setFetch(previous);
  }
});
