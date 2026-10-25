// The visitor code (#1553): a random UUID in a first-party cookie, one per
// browser, kept a year. It names a browser, not a person, until a Google
// sign-in links it to the account (the server reads it at the callback, which
// is why it is a cookie and not `localStorage`). `SameSite=Lax`, so the
// return from Google, a top-level navigation, carries it.

/** The cookie's name, as the server reads it (`crates/server/src/ab`). */
export const VISITOR_COOKIE = "scorsese_visitor";

const YEAR = 365 * 24 * 60 * 60;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** The code this page made, for a browser that would not keep the cookie. */
let made: string | null = null;

/**
 * This browser's code, made and kept now if it had none. Where the cookie
 * cannot be kept, the page still uses one code for all it sends.
 */
export function visitorCode(): string {
  const kept = document.cookie
    .split(";")
    .map((pair) => pair.trim())
    .find((pair) => pair.startsWith(`${VISITOR_COOKIE}=`))
    ?.slice(VISITOR_COOKIE.length + 1)
    .toLowerCase();
  if (kept && UUID.test(kept)) return kept;
  if (made) return made;
  const code = crypto.randomUUID();
  made = code;
  document.cookie = `${VISITOR_COOKIE}=${code}; Path=/; Max-Age=${YEAR}; SameSite=Lax; Secure`;
  return code;
}
