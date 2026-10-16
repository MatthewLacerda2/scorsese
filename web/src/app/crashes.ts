// Every crash the page has reaches the server's log (#1428). On 2026-10-16 the
// editor went black for every Portuguese and Spanish user for twelve days and
// the server never knew: every request it saw answered 200. Now whatever an
// error boundary catches, and whatever throws or rejects with nothing to catch
// it, is posted to `POST /api/client-errors`, once.
//
// What is sent says where and what, never who: the message, the stacks cut
// short, the page's path, its language and which build was running.

import { api, type ClientError } from "@/api";
import { initialLanguage } from "@/i18n/language";

/** Which build is running: the minute `vite build` ran, or `dev`. */
const BUILD: string = import.meta.env.VITE_BUILD ?? "dev";

/** Lines of a stack kept: where it threw, and enough of the way there. */
const STACK_LINES = 20;

/** Characters any one field keeps, so a report stays under the server's 16 KB. */
const FIELD = 4000;

/** Reports one page load sends: a crash in a loop has been told by then. */
const MOST = 10;

/** Report one crash: what threw, and React's component stack when it has one. */
type Report = (error: unknown, componentStack?: string | null) => void;

/**
 * A reporter sending through `send`, each thrown object once and at most
 * `most` in all. The page has one ({@link reportCrash}); a test makes its own.
 */
export function reporter(
  send: (report: ClientError) => Promise<unknown> = api.crashes.report,
  most = MOST,
): Report {
  const seen = new WeakSet<object>();
  let sent = 0;
  return (error, componentStack) => {
    if (typeof error === "object" && error !== null) {
      // A boundary's catch and the window's handler can both see one error.
      if (seen.has(error)) return;
      seen.add(error);
    }
    if (sent >= most) return;
    sent += 1;
    try {
      send(describe(error, componentStack)).catch(() => {});
    } catch {
      // Reporting a crash must never be the next one.
    }
  };
}

/** The page's reporter. */
export const reportCrash: Report = reporter();

/**
 * Report what throws or rejects with nothing to catch it, from now on.
 * Returns the way to stop.
 */
export function listenForCrashes(target: Window = window, report: Report = reportCrash) {
  const thrown = (event: ErrorEvent) => report(event.error ?? event.message);
  const rejected = (event: PromiseRejectionEvent) => report(event.reason);
  target.addEventListener("error", thrown);
  target.addEventListener("unhandledrejection", rejected);
  return () => {
    target.removeEventListener("error", thrown);
    target.removeEventListener("unhandledrejection", rejected);
  };
}

/** The report for `error`, every field cut to size. */
function describe(error: unknown, componentStack?: string | null): ClientError {
  const thrown = error instanceof Error ? error : null;
  return {
    message: cut(thrown ? `${thrown.name}: ${thrown.message}` : String(error)),
    stack: shorten(thrown?.stack),
    component_stack: shorten(componentStack ?? undefined),
    path: window.location.pathname,
    language: initialLanguage(),
    build: BUILD,
  };
}

/** The first {@link STACK_LINES} lines of `stack`, cut to size. */
function shorten(stack: string | undefined): string | undefined {
  if (!stack) return undefined;
  return cut(stack.trim().split("\n").slice(0, STACK_LINES).join("\n"));
}

function cut(text: string): string {
  return text.length > FIELD ? `${text.slice(0, FIELD)}…` : text;
}
