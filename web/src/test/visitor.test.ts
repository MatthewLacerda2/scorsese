// `forgetVisitor` leaves no visitor cookie, even when it runs in the same
// millisecond the cookie was set (happy-dom keeps a `Max-Age=0` cookie until
// the clock moves on, #1696).

import { expect, test } from "bun:test";
import { VISITOR_COOKIE } from "@/ab/visitor";
import { forgetVisitor, seeVersion } from "./visitor";

test("forgetVisitor removes the cookie at once, however often it is set", () => {
  for (let i = 0; i < 500; i += 1) {
    seeVersion("hero-subtitle", "a");
    forgetVisitor();
    expect(document.cookie).not.toContain(`${VISITOR_COOKIE}=`);
  }
});
