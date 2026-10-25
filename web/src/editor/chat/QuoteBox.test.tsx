// The quote box, read in a glance (#1573): each item's name in its kind's hue
// with its prompt under it, one cost sentence, Confirm / Decline, and a change
// box whose button shows only once something is typed. With a half-price batch
// beside the price for now (#947): both prices, and two yeses, neither of them
// the default.

import { expect, test } from "bun:test";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderToString } from "react-dom/server";
import type { EditorProject } from "@/api";
import type { QuoteView } from "@/api/chat";
import { queryKeys } from "@/app/queryKeys";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { QuoteBox } from "./QuoteBox";

const box = CATALOGUES.en.chat.quote;

const quote: QuoteView = {
  tool: "generate",
  items: [
    {
      subject: "pose-wide-senior",
      says: "$1.20: 1 still at $1.20",
      brief: "prompt",
      description: "A senior on a park bench, wide.",
    },
    {
      subject: "rain-loop",
      says: "$0.12: 1 sound at $0.12",
      brief: "prompt",
      description: "Soft rain on a tin roof.",
    },
  ],
  lines: ["Total: $1.32", "A line no item claims"],
  micros: 1_320_000,
  expires_at: Date.now() / 1000 + 900,
};

/** The editor's project, holding both items as sketches, and never fetched again. */
function client(): QueryClient {
  const queries = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } });
  queries.setQueryData(queryKeys.editor(7), {
    document: {
      assets: [
        { id: "pose-wide-senior", kind: "generated_image", state: "sketch" },
        { id: "rain-loop", kind: "generated_audio", state: "sketch" },
      ],
    },
  } as unknown as EditorProject);
  return queries;
}

function tree(view: QuoteView) {
  return (
    <QueryClientProvider client={client()}>
      <I18nProvider initial="en">
        <QuoteBox turn={1} project={7} quote={view} />
      </I18nProvider>
    </QueryClientProvider>
  );
}

test("two items read as two coloured names, their prompts, and one cost sentence", () => {
  render(tree(quote));
  const still = screen.getByText("pose-wide-senior");
  const sound = screen.getByText("rain-loop");
  expect(still.style.color).toBe("var(--kind-generated-image)");
  expect(sound.style.color).toBe("var(--kind-generated-audio)");
  expect(screen.getByText("A senior on a park bench, wide.")).toBeTruthy();
  expect(screen.getByText("Soft rain on a tin roof.")).toBeTruthy();
  expect(screen.getByText(box.ask("$1.32"))).toBeTruthy();
  const page = document.body.textContent ?? "";
  expect(page).not.toContain("$1.20");
  expect(page).not.toContain("A line no item claims");
  expect(page).not.toContain("Prompt:");
  expect(screen.getByRole("button", { name: box.confirm })).toBeTruthy();
  expect(screen.getByRole("button", { name: box.decline })).toBeTruthy();
});

test("the change button shows only once something is typed", async () => {
  render(tree(quote));
  expect(screen.queryByRole("button", { name: box.change })).toBeNull();
  await userEvent.type(screen.getByPlaceholderText(box.changePlaceholder), "make it night");
  expect(screen.getByRole("button", { name: box.change })).toBeTruthy();
});

test("a batch offered beside the quote is a second yes, priced", () => {
  const page = renderToString(tree({ ...quote, batch_micros: 660_000 }));
  expect(page).toContain(box.choose("$1.32", "$0.66"));
  expect(page).toContain(box.now("$1.32"));
  expect(page).toContain(box.batch("$0.66"));
  expect(page).not.toContain(`>${box.confirm}<`);
});

test("a quote with no offer asks the one question", () => {
  const page = renderToString(tree(quote));
  expect(page).toContain(box.confirm);
  expect(page).not.toContain(box.batch("$0.66"));
});
