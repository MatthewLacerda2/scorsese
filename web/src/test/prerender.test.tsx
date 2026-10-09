// A server render that suspends under a router leaves nothing behind for a DOM
// test of a router in the same process (#963) — `prerenderHtml` resets it.

import { expect, spyOn, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { lazy, Suspense } from "react";
import { prerender } from "react-dom/static";
import { Link, MemoryRouter } from "react-router";
import { prerenderHtml } from "@/test/prerender";

// A page that loads like a lazy route (#896): suspends once, then draws.
const Page = lazy(async () => ({ default: () => <p>page</p> }));

const routed = (
  <MemoryRouter>
    <Suspense fallback="loading">
      <Page />
    </Suspense>
  </MemoryRouter>
);

function mountRouter() {
  render(
    <MemoryRouter>
      <Link to="/">home</Link>
    </MemoryRouter>,
  );
  return screen.getByText("home").getAttribute("href");
}

test("a bare prerender of a suspending page under a router leaks the router", async () => {
  // The cause, pinned: if this stops throwing, React fixed it upstream and the
  // reset in `prerenderHtml` can go.
  // It also keeps the test below honest: the page really suspends.
  const warn = spyOn(console, "error").mockImplementation(() => {});
  try {
    await new Response((await prerender(routed)).prelude).text();
    expect(mountRouter).toThrow("inside another <Router>");
  } finally {
    await prerender(null);
    warn.mockRestore();
  }
});

test("after prerenderHtml, a DOM test mounts a router of its own", async () => {
  expect(await prerenderHtml(routed)).toContain("<p>page</p>");
  expect(mountRouter()).toBe("/");
});
