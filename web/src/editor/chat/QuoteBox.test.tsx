// The quote box with a half-price batch beside the price for now (#947): both
// prices, and two yeses, neither of them the default.

import { expect, test } from "bun:test";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import type { QuoteView } from "@/api/chat";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { QuoteBox } from "./QuoteBox";

const quote: QuoteView = {
  tool: "generate",
  lines: [],
  micros: 1_320_000,
  expires_at: Date.now() / 1000 + 900,
};

function html(view: QuoteView): string {
  return renderToString(
    <QueryClientProvider client={new QueryClient()}>
      <I18nProvider initial="en">
        <QuoteBox turn={1} quote={view} />
      </I18nProvider>
    </QueryClientProvider>,
  );
}

test("a batch offered beside the quote is a second yes, priced", () => {
  const box = CATALOGUES.en.chat.quote;
  const page = html({ ...quote, batch_micros: 660_000 });
  expect(page).toContain(box.now("$1.32"));
  expect(page).toContain(box.batch("$0.66"));
  expect(page).not.toContain(`>${box.confirm}<`);
});

test("a quote with no offer asks the one question", () => {
  const box = CATALOGUES.en.chat.quote;
  const page = html(quote);
  expect(page).toContain(box.confirm);
  expect(page).not.toContain(box.batch("$0.66"));
});
