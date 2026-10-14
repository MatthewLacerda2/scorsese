// "Add credits" (#548): what the reais buy is shown before paying, and Pay is
// offered only for an amount the server will take.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import type { PaymentQuote } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { AddCreditsDialog } from "./AddCredits";

const FIFTY: PaymentQuote = {
  brl_centavos: 5_000,
  credit_micros: 10_020_040,
  brl_per_usd: "4.99",
  rate_source: "awesomeapi",
  minimum_centavos: 2_000,
  maximum_centavos: 200_000,
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
  for (const value of ["abc", "10", "2.000,01"]) {
    fireEvent.change(field, { target: { value } });
    expect(screen.queryByText(/in credits/)).toBeNull();
    const pay = screen.getByRole("button", { name: "Pay" }) as HTMLButtonElement;
    expect(pay.disabled).toBe(true);
  }
});
