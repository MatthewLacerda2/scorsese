// A piece that throws while rendering leaves the page saying so, with a way
// back, rather than the blank page React leaves when nothing catches it.

import { afterEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import { messagesFor } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { ErrorBoundary } from "./ErrorBoundary";

afterEach(cleanup);

function Broken(): never {
  throw new Error("a broken piece");
}

test("a crash under the boundary shows the page's crash words and a reload", () => {
  const quiet = console.error;
  console.error = () => {};
  try {
    render(
      <I18nProvider initial="pt-BR">
        <ErrorBoundary>
          <Broken />
        </ErrorBoundary>
      </I18nProvider>,
    );
  } finally {
    console.error = quiet;
  }
  const words = messagesFor("pt-BR").common.crashed;
  expect(screen.getByText(words.text)).toBeTruthy();
  expect(screen.getByRole("button", { name: words.reload })).toBeTruthy();
});
