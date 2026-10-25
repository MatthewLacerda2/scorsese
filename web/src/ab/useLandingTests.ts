// The landing page's half of the A/B tests (#1553): draw this browser into the
// running test, say so with a `view`, and keep count of what the visit does —
// the deepest section reached (`section`, and `end` for the last), and the
// seconds the page was visible (`time`, sent each time it is hidden or left,
// at most half an hour a visit). `prompt` is the landing's to send, when a
// prompt box is sent (`sendPrompt`). With no test running nothing is drawn,
// recorded or kept, the cookie included.
//
// The sections are whatever carries `data-ab-section` (`ScrollIn` does), in
// the page's order, so adding or moving one needs nothing here. One counts as
// reached once it comes up into the window's top half, so a tall window does
// not reach the last section on load.

import { useEffect, useMemo } from "react";
import { send, versions } from "./record";
import { type AbTest, runningTests } from "./tests";
import { visitorCode } from "./visitor";

const MOST_SECONDS = 30 * 60;

/**
 * Draws the visit and returns the version of each running test; records it
 * while `counted` (never for a signed-in visitor, nor before that is known).
 * `running` is the tests drawn into; this build's own unless a test says.
 */
export function useLandingTests(
  counted: boolean,
  running?: readonly AbTest[],
): Record<string, string> {
  const shown = useMemo(() => {
    const tests = running ?? runningTests();
    return tests.length === 0 ? {} : versions(visitorCode(), tests);
  }, [running]);
  const testing = Object.keys(shown).length > 0;

  useEffect(() => {
    if (!counted || !testing) return;
    send([{ kind: "view" }], shown);
    const sections = [...document.querySelectorAll("[data-ab-section]")];
    let deepest = 0;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const index = sections.indexOf(entry.target) + 1;
          if (!entry.isIntersecting || index <= deepest) continue;
          deepest = index;
          const reached = { kind: "section", value: index } as const;
          send(index === sections.length ? [reached, { kind: "end" }] : [reached]);
        }
      },
      { rootMargin: "0px 0px -50% 0px" },
    );
    for (const section of sections) observer.observe(section);

    let since = document.visibilityState === "visible" ? performance.now() : null;
    let visible = 0;
    const sendTime = () => {
      if (since === null) return;
      const elapsed = Math.round((performance.now() - since) / 1000);
      const seconds = Math.min(elapsed, MOST_SECONDS - visible);
      since = null;
      if (seconds <= 0) return;
      visible += seconds;
      send([{ kind: "time", value: seconds }]);
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible") since = performance.now();
      else sendTime();
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
      // Left for another page of the app: the time so far still counts.
      sendTime();
    };
  }, [shown, counted, testing]);

  return shown;
}

/** Records that a prompt box was sent, while a test runs. */
export function sendPrompt(running: readonly AbTest[] = runningTests()) {
  if (running.length > 0) send([{ kind: "prompt" }]);
}
