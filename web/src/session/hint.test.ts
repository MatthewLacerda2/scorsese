// The hint `/` decides its first paint by (#1246): read as the server sets it,
// dropped when the session answers as nobody.

import { afterEach, expect, test } from "bun:test";
import { forgetSignedInHint, HINT_COOKIE, signedInHint } from "@/session/hint";

afterEach(forgetSignedInHint);

test("no hint, no session assumed", () => {
  document.cookie = "scorsese_signed_in_elsewhere=1; Path=/";
  expect(signedInHint()).toBe(false);
  document.cookie = "scorsese_signed_in_elsewhere=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
});

test("the server's hint is read, among other cookies, and forgetting it drops it", () => {
  document.cookie = "theme=dark; Path=/";
  document.cookie = `${HINT_COOKIE}=1; Path=/; Secure; SameSite=Strict`;
  expect(signedInHint()).toBe(true);
  forgetSignedInHint();
  expect(signedInHint()).toBe(false);
  expect(document.cookie).toContain("theme=dark");
  document.cookie = "theme=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
});
