// A piece that throws while rendering leaves the page saying so, with a way
// back, rather than the blank page React leaves when nothing catches it; a
// panel that throws says so in its own place and leaves its siblings drawn
// (#1428). Either way the crash is reported to the server.

import { afterEach, beforeEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import { setFetch } from "@/api/client";
import { messagesFor } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { ErrorBoundary } from "./ErrorBoundary";

let posted: string[] = [];
let previous: Parameters<typeof setFetch>[0];

beforeEach(() => {
  posted = [];
  previous = setFetch(async (url) => {
    posted.push(url);
    return new Response(null, { status: 204 });
  });
});

afterEach(() => {
  cleanup();
  setFetch(previous);
});

function Broken(): never {
  throw new Error("a broken piece");
}

/** Render `tree` without React's own logging of the error it catches. */
function quietly(tree: React.ReactNode) {
  const quiet = console.error;
  console.error = () => {};
  try {
    render(<I18nProvider initial="pt-BR">{tree}</I18nProvider>);
  } finally {
    console.error = quiet;
  }
}

test("a crash under the boundary shows the page's crash words and a reload", async () => {
  quietly(
    <ErrorBoundary>
      <Broken />
    </ErrorBoundary>,
  );
  const words = messagesFor("pt-BR").common.crashed;
  expect(screen.getByText(words.text)).toBeTruthy();
  expect(screen.getByRole("button", { name: words.reload })).toBeTruthy();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(posted).toEqual(["/api/client-errors"]);
});

test("a panel that throws says so in its place, and its siblings stay drawn", () => {
  quietly(
    <>
      <ErrorBoundary place="panel">
        <p>the chat</p>
      </ErrorBoundary>
      <ErrorBoundary place="panel">
        <Broken />
      </ErrorBoundary>
      <ErrorBoundary place="panel">
        <p>the timeline</p>
      </ErrorBoundary>
    </>,
  );
  const words = messagesFor("pt-BR").common.crashed;
  expect(screen.getByText("the chat")).toBeTruthy();
  expect(screen.getByText("the timeline")).toBeTruthy();
  expect(screen.getByText(words.part)).toBeTruthy();
  expect(screen.queryByText(words.text)).toBeNull();
  expect(screen.getAllByRole("button", { name: words.reload }).length).toBe(1);
});
