// Queries more than one page reads, under the keys app/queryClient.ts
// describes. A page-specific query stays with its page.

import { useQuery } from "@tanstack/react-query";
import { api, type LibraryFilter } from "@/api";

/** The user's projects, newest write first. */
export function useProjects() {
  return useQuery({ queryKey: ["projects"], queryFn: api.projects.list });
}

/** The user's balance and the display rate — the header shows it everywhere. */
export function useBalance() {
  return useQuery({ queryKey: ["credits", "balance"], queryFn: api.credits.balance });
}

/** A page of library tiles, narrowed by `filter`. */
export function useLibrary(filter: LibraryFilter) {
  return useQuery({
    queryKey: ["library", "list", filter],
    queryFn: () => api.library.list(filter),
  });
}

/** Everything known about one library file; idle while `id` is null. */
export function useLibraryItem(id: number | null) {
  return useQuery({
    queryKey: ["library", "item", id],
    queryFn: () => api.library.details(id as number),
    enabled: id !== null,
  });
}
