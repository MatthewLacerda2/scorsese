// What the landing page does to turn a visit into a start (#1566): it says the
// start is free, with the day's allowance read from the server and never
// written in a string; its header leads nowhere but pricing and the way in;
// and a third prompt box, after the film, hands off like the other two.

import { afterEach, expect, test } from "bun:test";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import type { Prices } from "@/api";
import { setFetch } from "@/api/client";
import { createQueryClient, ME } from "@/app/queryClient";
import type { Language } from "@/i18n/language";

const PRICES: Prices = {
  examples: [],
  free: { daily_micros: 4_000_000, monthly_micros: 30_000_000 },
  recharge: null,
};

/** The landing page signed out, `GET /api/prices` answered by `prices` (`null`: it fails). */
function visit(prices: Prices | null, language: Language = "en") {
  setFetch(async (input) => {
    if (String(input).endsWith("/api/prices") && prices) return Response.json(prices);
    return Response.json({ error: "no" }, { status: 500 });
  });
  const client = createQueryClient();
  client.setQueryData(ME, null);
  client.setQueryData(["formats"], { video: [], image: [], audio: [], html: [], text: [] });
  render(
    <App
      language={language}
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={["/"]}>{routes}</MemoryRouter>}
    />,
  );
}

const real = setFetch((input, init) => fetch(input, init));
afterEach(() => setFetch(real));

test("under the first box, the free start says the day's allowance the server names", async () => {
  visit(PRICES);
  expect(await screen.findByText("Start free: US$4 a day, no card.")).toBeDefined();
});

test("the free line is said in each language, with the server's figure", async () => {
  visit({ ...PRICES, free: { ...PRICES.free, daily_micros: 2_500_000 } }, "pt-BR");
  expect(await screen.findByText("Comece grátis: US$ 2.50 por dia, sem cartão.")).toBeDefined();
});

test("in Spanish too", async () => {
  visit(PRICES, "es");
  expect(await screen.findByText("Empieza gratis: US$ 4 al día, sin tarjeta.")).toBeDefined();
});

test("without the prices, the free line stands without a figure rather than a wrong one", async () => {
  visit(null);
  expect(await screen.findByText("Start free, no card.")).toBeDefined();
  expect(document.body.textContent).not.toContain("US$");
});

test("the header leads only to pricing and the way in; the footer keeps the documents", async () => {
  visit(PRICES);
  const header = await waitFor(() => {
    const found = document.querySelector("header");
    if (!found) throw new Error("no header yet");
    return found;
  });
  const links = [...header.querySelectorAll("a")].map((a) => a.getAttribute("href"));
  expect(links).toEqual(["/", "/pricing"]);
  const footer = document.querySelector("footer");
  const kept = [...(footer?.querySelectorAll("a") ?? [])].map((a) => a.getAttribute("href"));
  expect(kept).toContain("/terms");
  expect(kept).toContain("/privacy");
});

test("a third box, after the film, hands its prompt off like the others", async () => {
  const went: string[] = [];
  const assign = window.location.assign;
  window.location.assign = (url: string | URL) => void went.push(String(url));
  try {
    visit(PRICES);
    const ask = await screen.findByText("Your turn. Describe the video you need.");
    const film = screen.getByLabelText("Videos made with scorsese", { selector: "video" });
    expect(film.compareDocumentPosition(ask) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const boxes = screen.getAllByRole("textbox", { name: "Describe your video" });
    expect(boxes.length).toBe(3);
    const middle = boxes[1] as HTMLElement;
    expect(ask.compareDocumentPosition(middle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await userEvent.type(middle, "a teaser for my band{Enter}");
    await waitFor(() => expect(went).toEqual(["/api/auth/google?next=%2Fstart"]));
  } finally {
    window.location.assign = assign;
  }
});
