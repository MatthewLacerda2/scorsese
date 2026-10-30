// Who is logged in, and the guard that sends everyone else to the login page.
//
// The session itself is an HttpOnly cookie the page cannot read (docs/web.md,
// *Accounts*), so "am I logged in?" is asked of the server: `GET /api/me`,
// cached under `["me"]`. `null` there means logged out — set by a `401` from
// any request (app/queryClient.ts) as well as by logging out here. A `401`
// here also drops the readable hint that a session exists (./hint.ts).

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Navigate, Outlet, useLocation } from "react-router";
import { type Account, ApiError, api } from "@/api";
import { ME } from "@/app/queryClient";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { forgetSignedInHint } from "@/session/hint";

async function currentAccount(): Promise<Account | null> {
  try {
    return await api.account.me();
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      forgetSignedInHint();
      return null;
    }
    throw error;
  }
}

/** The logged-in account: `null` when logged out, `undefined` while asking. */
export function useAccount() {
  return useQuery({ queryKey: ME, queryFn: currentAccount, staleTime: Number.POSITIVE_INFINITY });
}

/** Log out, and forget everything cached for this account. */
export function useLogout() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.account.logout,
    onSettled: () => {
      queryClient.clear();
      queryClient.setQueryData(ME, null);
    },
  });
}

/**
 * Where to go after logging in: `next` when it is a path on this site, `/`
 * otherwise — never another origin, so a crafted login link cannot bounce a
 * user somewhere else (`//evil.example` is a URL to another host). No ASCII
 * control character or space anywhere, either: a browser drops a tab or a
 * newline from a URL, so `/\t/evil.example` is `//evil.example`. The server's
 * `landing` is the same rule.
 */
export function safeNext(next: string | null): string {
  if (
    !next?.startsWith("/") ||
    next.startsWith("//") ||
    next.startsWith("/\\") ||
    [...next].some(isControlOrSpace)
  ) {
    return "/";
  }
  return next;
}

/** An ASCII control character (U+0000–U+001F, U+007F) or a space. */
function isControlOrSpace(character: string): boolean {
  const code = character.charCodeAt(0);
  return code <= 0x20 || code === 0x7f;
}

/** A route guard: renders the page when logged in, the login page when not. */
export function RequireSession() {
  const account = useAccount();
  const t = useT();
  const location = useLocation();
  if (account.isPending) {
    return <p className="p-6 text-muted-foreground">{t.common.loading}</p>;
  }
  if (account.isError) {
    return (
      <div className="flex flex-col items-start gap-3 p-6">
        <p>{t.common.unreachable(account.error.message)}</p>
        <Button variant="outline" onClick={() => account.refetch()}>
          {t.common.tryAgain}
        </Button>
      </div>
    );
  }
  if (!account.data) {
    const next = encodeURIComponent(location.pathname + location.search);
    return <Navigate to={`/login?next=${next}`} replace />;
  }
  return <Outlet />;
}
