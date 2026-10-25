// The landing page's half of the A/B tests (#1553): draw this browser into the
// running test, say so with a `view`, and keep count of what the visit does —
// the deepest section reached (`section`, and `end` for the last), and the
// seconds the page was visible (`time`, sent each time it is hidden, at most
// half an hour a visit). `prompt` is the landing's to send, when a prompt
// box is sent (`sendPrompt`).
//
// The sections are whatever carries `data-ab-section` (`ScrollIn` does), in
// the page's order, so adding or moving one needs nothing here.

import { useEffect, useMemo } from "react";
import { send, versions } from "./record";
import { runningTests } from "./tests";
import { visitorCode } from "./visitor";

const MOST_SECONDS = 30 * 60;

/**
 * Draws the visit and returns the version of each running test; records it
 * while `counted` (not for a signed-in visitor on their way past).
 */
export function useLandingTests(counted: boolean): Record<string, string> {
  const shown = useMemo(() => versions(visitorCode(), runningTests()), []);

  useEffect(() => {
    if (!counted) return;
    send([{ kind: "view" }], shown);
    const sections = [...document.querySelectorAll("[data-ab-section]")];
    let deepest = 0;
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        const index = sections.indexOf(entry.target) + 1;
        if (!entry.isIntersecting || index <= deepest) continue;
        deepest = index;
        send(
          index === sections.length
            ? [{ kind: "section", value: index }, { kind: "end" }]
            : [{ kind: "section", value: index }],
        );
      }
    });
    for (const section of sections) observer.observe(section);

    let since = document.visibilityState === "visible" ? performance.now() : null;
    let visible = 0;
    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        since = performance.now();
        return;
      }
      if (since === null) return;
      const seconds = Math.min(
        Math.round((performance.now() - since) / 1000),
        MOST_SECONDS - visible,
      );
      since = null;
      if (seconds <= 0) return;
      visible += seconds;
      send([{ kind: "time", value: seconds }]);
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [shown, counted]);

  return shown;
}

/** Records that a prompt box was sent. */
export function sendPrompt() {
  send([{ kind: "prompt" }]);
}
