// The DOM every `bun test` file runs with (#898), loaded by `bunfig.toml`'s
// preload before any test file. happy-dom puts `window`, `document` and the
// events on the global object, so `@testing-library/react` can mount a
// component and `@testing-library/user-event` can click, type and drag it.
//
// Pages rendered through `react-dom/server` are unaffected: they never touch
// the DOM. A test that must see no `window` (code written for a server render)
// says so itself rather than this file guessing.

import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register({ url: "http://localhost/" });

// Imported only now, because Testing Library reads `document` as it loads.
const { cleanup } = await import("@testing-library/react");
const { afterEach, beforeEach } = await import("bun:test");

// Every page test sees the landing's heading test as version `a`, today's page;
// a test of `b` calls `seeVersion` itself (src/ab/heading.test.tsx).
const { seeVersion } = await import("./visitor");
beforeEach(() => seeVersion("hero-title", "a"));

// Bun has no global `afterEach` for Testing Library to find on its own, so each
// test's mounted tree is unmounted here — and the storage it wrote cleared, so
// no test reads another's panel sizes or language.
afterEach(() => {
  cleanup();
  window.localStorage.clear();
});
