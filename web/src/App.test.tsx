// The smoke test: the app renders at all. It goes through
// `react-dom/server` so it needs no DOM, which keeps `bun test` free of a
// browser environment until a test genuinely needs one — a page that handles
// clicks is the point to add one, not this.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { App } from "@/App";

test("the app renders its placeholder page", () => {
  const html = renderToString(<App />);
  expect(html).toContain("scorsese");
  expect(html).toContain('data-slot="button"');
});
