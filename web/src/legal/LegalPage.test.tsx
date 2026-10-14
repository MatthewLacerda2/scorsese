// The terms and the privacy policy (#547): public at their URLs, signed in or
// not, in Portuguese with a word to a visitor reading in another language,
// and linked from the landing page's footer and the sign-in popup.

import { expect, test } from "bun:test";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Account } from "@/api";
import { createQueryClient, ME } from "@/app/queryClient";
import type { Language } from "@/i18n/language";
import { SELLER } from "@/legal/seller";

const ana: Account = { id: 1, email: "ana@example.com", created_at: 0 };

function visit(url: string, account: Account | null, language: Language = "en") {
  const client = createQueryClient();
  client.setQueryData(ME, account);
  client.setQueryData(["credits", "balance"], { balance_micros: 0 });
  render(
    <App
      language={language}
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  );
}

test("/terms is public, and says why it is in Portuguese", async () => {
  visit("/terms", null);
  expect(await screen.findByRole("heading", { level: 1, name: "Termos de uso" })).toBeDefined();
  expect(screen.getByText(/in Portuguese, the language of the contract/)).toBeDefined();
  expect(
    screen.getByText(new RegExp(`CNPJ ${SELLER.cnpj.replaceAll(".", "\\.")} ·`)),
  ).toBeDefined();
});

test("/privacy is there for a signed-in visitor too, with no note in Portuguese", async () => {
  visit("/privacy", ana, "pt-BR");
  expect(
    await screen.findByRole("heading", { level: 1, name: "Política de privacidade" }),
  ).toBeDefined();
  expect(screen.queryByText(/idioma do contrato/)).toBeNull();
});

test("the landing page's footer links both, and names the seller", async () => {
  visit("/", null);
  // The footer is on the last slide, hidden until it is scrolled to, so it is
  // found by its markup rather than by what a screen reader would announce.
  const footer = await waitFor(() => {
    const found = document.querySelector("footer");
    if (!found?.textContent?.includes("Terms of use")) throw new Error("no footer yet");
    return found;
  });
  const links = [...footer.querySelectorAll("a")].map((a) => [
    a.textContent,
    a.getAttribute("href"),
  ]);
  expect(links).toContainEqual(["Terms of use", "/terms"]);
  expect(links).toContainEqual(["Privacy policy", "/privacy"]);
  expect(footer.textContent).toContain(`${SELLER.name} · CNPJ ${SELLER.cnpj}`);
});

test("the sign-in popup says signing in accepts both", async () => {
  visit("/login", null);
  const dialog = await screen.findByRole("dialog");
  expect(dialog.textContent).toContain("By continuing you accept the terms of use");
  expect(screen.getByRole("link", { name: "terms of use" }).getAttribute("href")).toBe("/terms");
  expect(screen.getByRole("link", { name: "privacy policy" }).getAttribute("href")).toBe(
    "/privacy",
  );
});
