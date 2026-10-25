// With no test running (#1553), the landing draws nothing, sends nothing and
// keeps no visitor cookie.

import { expect, test } from "bun:test";
import { render } from "@testing-library/react";
import type { AbTest } from "./tests";
import { sendPrompt, useLandingTests } from "./useLandingTests";
import { VISITOR_COOKIE } from "./visitor";

const NONE: readonly AbTest[] = [];

function Landing() {
  const shown = useLandingTests(true, NONE);
  return <p>{JSON.stringify(shown)}</p>;
}

test("no running test, no cookie and no beacon", () => {
  document.cookie = `${VISITOR_COOKIE}=; Path=/; Max-Age=0`;
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
