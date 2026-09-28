// The one cache every page reads the server through (TanStack Query).
//
// Query keys start with the area they belong to — `["me"]`, `["projects"]`,
// `["library", …]`, `["credits", …]` — so a change invalidates its whole area
// with one call and nothing else. A `401` from any request means the session
// ended (logged out elsewhere, expired, reset by the operator): the cached
// account is dropped, and the session guard sends the page to the login.

import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { ApiError } from "@/api";

/** The key the logged-in account is cached under. */
export const ME = ["me"] as const;

function loggedOut(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}

export function createQueryClient(): QueryClient {
  const client: QueryClient = new QueryClient({
    queryCache: new QueryCache({
      onError: (error) => {
        if (loggedOut(error)) client.setQueryData(ME, null);
      },
    }),
    mutationCache: new MutationCache({
      onError: (error) => {
        if (loggedOut(error)) client.setQueryData(ME, null);
      },
    }),
    defaultOptions: {
      queries: {
        // A refusal is an answer, not a blip: retrying a 404 or a 401 only
        // delays showing it. Network failures (status 0) and 5xx get one more try.
        retry: (count, error) =>
          count < 1 && (!(error instanceof ApiError) || error.status === 0 || error.status >= 500),
        staleTime: 10_000,
      },
    },
  });
  return client;
}
