// Every TanStack Query key the app uses, in one place.
//
// A key starts with the area it belongs to, so invalidating the area's bare
// key (`queryKeys.projects`) refetches everything under it — the editor's
// project, its renders and its cached stills included. Nothing outside this
// file writes a key as an array literal.

import type { HistoryFilter, LibraryFilter } from "@/api";

export const queryKeys = {
  me: ["me"] as const,

  projects: ["projects"] as const,
  project: (id: number) => ["projects", id] as const,
  /** The project the editor works on. */
  editor: (id: number) => ["projects", "editor", id] as const,
  renders: (id: number) => ["projects", "renders", id] as const,
  /** The project's script, and the revision it was read at (#1445). */
  script: (id: number) => ["projects", "script", id] as const,
  /** A frame of one revision of a project, rendered at a raster. */
  still: (id: number, revision: number, frame: number, raster: string) =>
    ["projects", "still", id, revision, frame, raster] as const,

  library: ["library"] as const,
  libraryList: (filter: LibraryFilter) => ["library", "list", filter] as const,
  libraryItem: (id: number | null) => ["library", "item", id] as const,

  credits: ["credits"] as const,
  balance: ["credits", "balance"] as const,
  history: (filter: Omit<HistoryFilter, "before" | "limit">) =>
    ["credits", "history", filter] as const,

  /** Every conversation, or (given a project) that project's. */
  chat: ["chat"] as const,
  conversation: (projectId: number) => ["chat", projectId] as const,

  styles: ["styles"] as const,

  paymentQuote: (centavos: number | null) => ["payments", "quote", centavos] as const,
  paymentConfirm: (session: string) => ["payments", "confirm", session] as const,
};
