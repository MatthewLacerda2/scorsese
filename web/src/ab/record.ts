// What the landing tells the server about a visit (#1553), through
// `api.ab.send`: the versions shown, and what happened. A browser driven by a
// script (`navigator.webdriver`) or one that says it is a bot sends nothing,
// and neither does a page whose JavaScript never ran, which leaves the
// scanners out. Sending never fails a page: the beacon is fire and forget.

import { type AbEvent, api } from "@/api";
import { draw } from "./draw";
import type { AbTest } from "./tests";
import { visitorCode } from "./visitor";

const BOT = /bot|crawl|spider|slurp|headless|lighthouse|facebookexternalhit/i;

/** Whether this browser is one whose visits are not counted. */
function automated(): boolean {
  return navigator.webdriver === true || BOT.test(navigator.userAgent);
}

/** The version of each of `tests` this browser sees, by test id. */
export function versions(code: string, tests: readonly AbTest[]): Record<string, string> {
  return Object.fromEntries(tests.map((test) => [test.id, draw(code, test)]));
}

/** Tell the server `events` happened, with the versions shown when `shown` is given. */
export function send(events: AbEvent[], shown?: Record<string, string>) {
  if (automated()) return;
  try {
    api.ab.send({
      visitor: visitorCode(),
      exposures: Object.entries(shown ?? {}).map(([test, version]) => ({ test, version })),
      events,
    });
  } catch {
    // A blocked beacon or cookie loses one count, never the page.
  }
}
