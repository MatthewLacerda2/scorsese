// Queries more than one page reads, under the keys app/queryClient.ts
// describes. A page-specific query stays with its page.

import { useQuery } from "@tanstack/react-query";
import { api, type LibraryFilter } from "@/api";
import { queryKeys } from "@/app/queryKeys";

/** The user's projects, newest write first. */
export function useProjects() {
  return useQuery({ queryKey: queryKeys.projects, queryFn: api.projects.list });
}

/** The user's balance and the display rate — the header shows it everywhere. */
export function useBalance() {
  return useQuery({ queryKey: queryKeys.balance, queryFn: api.credits.balance });
}

/** A page of library tiles, narrowed by `filter`. */
export function useLibrary(filter: LibraryFilter) {
  return useQuery({
    queryKey: queryKeys.libraryList(filter),
    queryFn: () => api.library.list(filter),
  });
}

/** Everything known about one library file; idle while `id` is null. */
export function useLibraryItem(id: number | null) {
  return useQuery({
    queryKey: queryKeys.libraryItem(id),
    queryFn: () => api.library.details(id as number),
    enabled: id !== null,
  });
}

/**
 * The extensions the library takes, by kind (#1534): what a picker hints and a
 * drop is refused by. The same for everyone and for the life of the page, and
 * asked only once `enabled`, so a page that never attaches never asks.
 */
export const formatsQuery = {
  queryKey: queryKeys.formats,
  queryFn: api.library.formats,
  staleTime: Number.POSITIVE_INFINITY,
};

export function useFormats(enabled = true) {
  return useQuery({ ...formatsQuery, enabled });
}
