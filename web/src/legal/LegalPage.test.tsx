// The terms and the privacy policy (#547): public at their URLs, signed in or
// not, in the site's language (Portuguese is the binding text and the others say so),
// and linked from the landing page's footer and the sign-in popup.

import { expect, test } from "bun:test";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

test("/terms is public, in the site's language, with a summary and the note that Portuguese prevails", async () => {
  visit("/terms", null);
  expect(await screen.findByRole("heading", { level: 1, name: "Terms of use" })).toBeDefined();
  expect(document.querySelector("article")?.getAttribute("lang")).toBe("en");
  expect(screen.getByRole("heading", { name: "In short" })).toBeDefined();
  expect(screen.getByText(/the Portuguese text prevails/)).toBeDefined();
  expect(
    screen.getByText(new RegExp(`CNPJ ${SELLER.cnpj.replaceAll(".", "\\.")} ·`)),
  ).toBeDefined();
});

test("/privacy is there for a signed-in visitor too, in Portuguese with no note", async () => {
  visit("/privacy", ana, "pt-BR");
  expect(
    await screen.findByRole("heading", { level: 1, name: "Política de privacidade" }),
  ).toBeDefined();
  expect(screen.getByRole("heading", { name: "Em resumo" })).toBeDefined();
  expect(screen.queryByText(/o texto em português é o que vale/i)).toBeNull();
});

test("a translation can be switched to the Portuguese text and back, and the vendors link their policies", async () => {
  visit("/privacy", null, "es");
  expect(
    await screen.findByRole("heading", { level: 1, name: "Política de privacidad" }),
  ).toBeDefined();
  const policy = screen.getAllByRole("link", { name: "Política de privacidad" })[0];
  expect(policy?.getAttribute("href")).toBe("https://www.anthropic.com/legal/privacy");
  await userEvent.click(screen.getByRole("button", { name: "Leer el texto en portugués" }));
  expect(screen.getByRole("heading", { level: 1, name: "Política de privacidade" })).toBeDefined();
  expect(document.querySelector("article")?.getAttribute("lang")).toBe("pt-BR");
  await userEvent.click(screen.getByRole("button", { name: "Volver a la traducción" }));
  expect(screen.getByRole("heading", { level: 1, name: "Política de privacidad" })).toBeDefined();
});

test("the landing page's footer links pricing and both documents, and names the seller", async () => {
  visit("/", null);
  const footer = await waitFor(() => {
    const found = document.querySelector("footer");
    if (!found?.textContent?.includes("Terms of use")) throw new Error("no footer yet");
    return found;
  });
  const links = [...footer.querySelectorAll("a")].map((a) => [
    a.textContent,
    a.getAttribute("href"),
  ]);
  // The documents are the footer's alone: the header leads only to pricing (#1566).
  expect(links).toContainEqual(["Pricing", "/pricing"]);
  expect(links).toContainEqual(["Terms of use", "/terms"]);
  expect(links).toContainEqual(["Privacy policy", "/privacy"]);
  expect(footer.textContent).toContain(`${SELLER.name} · CNPJ ${SELLER.cnpj}`);
});

test("a document has the public pages' header: a way home, pricing, the language, a way in", async () => {
  visit("/terms", null);
  const header = await screen.findByRole("banner");
  expect(within(header).getByRole("link", { name: "scorsese" }).getAttribute("href")).toBe("/");
  expect(within(header).getByRole("link", { name: "Pricing" }).getAttribute("href")).toBe(
    "/pricing",
  );
  expect(within(header).getByRole("combobox", { name: "Language" })).toBeDefined();
  await userEvent.click(within(header).getByRole("button", { name: "Sign in" }));
  expect(await screen.findByRole("dialog")).toBeDefined();
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
