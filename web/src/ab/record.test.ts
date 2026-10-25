// What the landing sends (#1553): a batch naming the visitor and the versions
// shown, kept to one code per browser, and nothing from a driven browser.

import { afterEach, expect, test } from "bun:test";
import type { AbBatch } from "@/api";
import { send } from "./record";
import { VISITOR_COOKIE, visitorCode } from "./visitor";

const sent: AbBatch[] = [];
const real = navigator.sendBeacon;
navigator.sendBeacon = (_url, body) => {
  (body as Blob).text().then((text) => sent.push(JSON.parse(text)));
  return true;
};
afterEach(() => {
  sent.length = 0;
  Object.defineProperty(navigator, "webdriver", { value: false, configurable: true });
});

test("a browser keeps its visitor code", () => {
  const code = visitorCode();
  expect(code).toMatch(/^[0-9a-f-]{36}$/);
  expect(document.cookie).toContain(`${VISITOR_COOKIE}=${code}`);
  expect(visitorCode()).toBe(code);
});

test("a batch names the visitor, the versions and the events", async () => {
  send([{ kind: "view" }], { heading: "b" });
  await Bun.sleep(0);
  expect(sent).toEqual([
    {
      visitor: visitorCode(),
      exposures: [{ test: "heading", version: "b" }],
      events: [{ kind: "view" }],
    },
  ]);
});

test("a browser driven by a script sends nothing", async () => {
  Object.defineProperty(navigator, "webdriver", { value: true, configurable: true });
  send([{ kind: "prompt" }]);
  await Bun.sleep(0);
  expect(sent).toEqual([]);
  expect(real === navigator.sendBeacon).toBe(false);
});
