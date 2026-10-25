// A visitor code the landing's A/B tests draw into a chosen version (#1554), so
// a test that reads the page reads the same one every run.

import { draw } from "@/ab/draw";
import { TESTS } from "@/ab/tests";
import { VISITOR_COOKIE } from "@/ab/visitor";

/** Keep a visitor cookie that `test` draws into `version`; the page then shows that one. */
export function seeVersion(testId: string, version: string) {
  const test = TESTS.find((t) => t.id === testId);
  if (!test) throw new Error(`no test ${testId}`);
  for (let at = 0; ; at += 1) {
    const code = `00000000-0000-4000-8000-${String(at).padStart(12, "0")}`;
    if (draw(code, test) === version) {
      document.cookie = `${VISITOR_COOKIE}=${code}; Path=/`;
      return;
    }
  }
}
