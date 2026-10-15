// The header's balance (#1280): a click opens a modal with the balance, Add
// credits and a link to the history; the separate "+" button is gone.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { AddCreditsProvider } from "@/app/addCredits";
import { BalanceChip } from "@/app/BalanceChip";
import { createQueryClient } from "@/app/queryClient";

function chip() {
  const client = createQueryClient();
  client.setQueryData(["credits", "balance"], { balance_micros: 12_340_000 });
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
