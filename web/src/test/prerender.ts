// A server render for tests, through `react-dom/static`'s `prerender`, which
// waits for every Suspense boundary — so a lazy page (#896) is drawn, never its
// fallback — and then leaves nothing behind for the DOM tests that share the
// process (#963).
//
// What it would leave behind otherwise: React's server renderer writes a
// provider's value straight onto the context object (`_currentValue`), the same
// field the client renderer reads, and pops it when the provider's subtree is
// done. A subtree that suspends is retried later from a snapshot of the
// contexts around it, and once the last retry ends react-dom 19.3 restores the
// snapshot it started from only for a nested render — so the contexts above the
// last suspended subtree stay written. Under a `<MemoryRouter>` that is the
// router's own; every `bun test` file runs in one process, so a later
// Testing Library `render` of a router reads it as an enclosing router and
// throws "You cannot render a <Router> inside another <Router>".
//
// A render whose only work is its root starts from the root's empty snapshot,
// which pops every context back to its default — so one empty render after each
// real one is the reset.

import type { ReactNode } from "react";
import { prerender } from "react-dom/static";

/** `node` rendered to HTML once every Suspense boundary in it has resolved. */
export async function prerenderHtml(node: ReactNode): Promise<string> {
  try {
    const { prelude } = await prerender(node);
    return await new Response(prelude).text();
  } finally {
    await prerender(null);
  }
}
