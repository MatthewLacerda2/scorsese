// Whether this browser was signed in here: a hint, read before any request
// (#1246).
//
// The session cookie is HttpOnly, so only `GET /api/me` can say whether it is
// still good. The server sets a second cookie beside it at sign-in and clears
// it at logout (crates/server/src/http/auth.rs, `HINT_COOKIE`): readable,
// carrying nothing but `1`. `/` uses it to decide what to draw while the
// session is asked — wait for it, or the landing page at once — and never as
// proof of anything.

/** The hint's name, as the server sets it. */
export const HINT_COOKIE = "scorsese_signed_in";

/** Whether the hint is set: a session was opened in this browser and not logged out of. */
export function signedInHint(): boolean {
  if (typeof document === "undefined") return false;
  return document.cookie.split(";").some((pair) => pair.trim().startsWith(`${HINT_COOKIE}=`));
}

/**
 * Drop a hint the server no longer stands behind — the session expired, or was
 * ended elsewhere — so the next visit to `/` draws the landing page at once.
 */
export function forgetSignedInHint() {
  if (typeof document === "undefined") return;
  document.cookie = `${HINT_COOKIE}=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Secure; SameSite=Strict`;
}
