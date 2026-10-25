// The header's balance (#1280): a click opens a modal with the balance, Add
// credits and a link to the history; the separate "+" button is gone.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { Balance } from "@/api";
import { AddCreditsProvider } from "@/app/addCredits";
import { BalanceChip } from "@/app/BalanceChip";
import { createQueryClient } from "@/app/queryClient";

function chip(
  balance: Balance = { balance_micros: 12_340_000, free_micros: 0, free_renews_at: 0 },
) {
  const client = createQueryClient();
  client.setQueryData(["credits", "balance"], balance);
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <AddCreditsProvider>
          <BalanceChip />
        </AddCreditsProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

test("clicking the balance opens the credits modal with both ways on", () => {
  chip();
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "$12.34" }));
  expect(screen.getByRole("dialog")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add credits" })).toBeTruthy();
  const history = screen.getByRole("link", { name: "Spending history" });
  expect(history.getAttribute("href")).toBe("/spending");
});

test("what is free today shows beside the balance, and in the modal when it renews", () => {
  chip({ balance_micros: 0, free_micros: 3_200_000, free_renews_at: 1_793_502_000 });
  const button = screen.getByRole("button", { name: /\$0\.00/ });
  expect(button.textContent).toContain("$3.20 free today");
  fireEvent.click(button);
  expect(screen.getByRole("dialog").textContent).toContain("Free credit renews");
});

test("nothing free says nothing of it", () => {
  chip({ balance_micros: 5_000_000, free_micros: 0, free_renews_at: 1_793_502_000 });
  expect(screen.getByRole("button", { name: "$5.00" }).textContent).not.toContain("free");
});
