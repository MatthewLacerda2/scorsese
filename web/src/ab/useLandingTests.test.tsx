// With no test running (#1553), the landing draws nothing, sends nothing and
// keeps no visitor cookie.

import { expect, test } from "bun:test";
import { render } from "@testing-library/react";
import { forgetVisitor } from "@/test/visitor";
import type { AbTest } from "./tests";
import { sendPrompt, useLandingTests } from "./useLandingTests";
import { VISITOR_COOKIE } from "./visitor";

const NONE: readonly AbTest[] = [];

function Landing() {
  const shown = useLandingTests(true, NONE);
  return <p>{JSON.stringify(shown)}</p>;
}

test("no running test, no cookie and no beacon", () => {
  forgetVisitor();
  let beacons = 0;
  navigator.sendBeacon = () => {
    beacons += 1;
    return true;
  };
  const page = render(<Landing />);
  sendPrompt(NONE);
  expect(page.container.textContent).toBe("{}");
  page.unmount();
  expect(beacons).toBe(0);
  expect(document.cookie).not.toContain(`${VISITOR_COOKIE}=`);
});

// A visit counts once the page sees a person use it (#1797): the first trusted
// pointer, touch, wheel or key event sends `input`, once. A script's own events
// are not trusted, and send nothing.

const RUNNING: readonly AbTest[] = [{ id: "heading", versions: ["a", "b"], running: true }];

function Running() {
  useLandingTests(true, RUNNING);
  return null;
}

/** An event as a browser makes it for a person, which a script cannot forge. */
function trusted(event: Event): Event {
  Object.defineProperty(event, "isTrusted", { value: true });
  return event;
}

test("the first input a person makes sends `input`, once", async () => {
  // happy-dom says it is driven; a person's browser does not.
  Object.defineProperty(navigator, "webdriver", { value: false, configurable: true });
  const kinds: string[] = [];
  navigator.sendBeacon = (_url, body) => {
    (body as Blob).text().then((text) => {
      for (const event of JSON.parse(text).events) kinds.push(event.kind);
    });
    return true;
  };
  const page = render(<Running />);
  window.dispatchEvent(new Event("pointermove"));
  await Bun.sleep(0);
  expect(kinds).toEqual(["view"]);
  window.dispatchEvent(trusted(new Event("pointermove")));
  window.dispatchEvent(trusted(new Event("keydown")));
  await Bun.sleep(0);
  expect(kinds).toEqual(["view", "input"]);
  page.unmount();
  forgetVisitor();
  Object.defineProperty(navigator, "webdriver", { value: true, configurable: true });
});
