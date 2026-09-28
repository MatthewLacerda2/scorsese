// How a list of files can be ordered. The server answers newest first and a
// library is dozens to hundreds of items (docs/web.md), so any other order is
// applied here rather than asked of it.

import type { LibraryTile } from "@/api";

export const SORTS = {
  newest: "Newest first",
  oldest: "Oldest first",
  name: "Name",
  largest: "Largest first",
} as const;

export type Sort = keyof typeof SORTS;

const byName = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** `tiles` in `order`, as a new array. Ids grow with arrival, so they date a file. */
export function sortTiles(tiles: readonly LibraryTile[], order: Sort): LibraryTile[] {
  const sorted = [...tiles];
  switch (order) {
    case "newest":
      return sorted.sort((a, b) => b.id - a.id);
    case "oldest":
      return sorted.sort((a, b) => a.id - b.id);
    case "name":
      return sorted.sort((a, b) => byName.compare(a.name, b.name) || b.id - a.id);
    case "largest":
      return sorted.sort((a, b) => b.size_bytes - a.size_bytes || b.id - a.id);
  }
}
