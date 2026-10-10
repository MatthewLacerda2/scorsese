// What the editor's library modal may pick, and what a pick does (#702). A
// pick brings the file into the project's assets (`import`, which never adds
// a file twice) without placing it: the user drags it onto a lane from there.

import type { LibraryTile } from "@/api";
import type { Messages } from "@/i18n/catalogue";
import type { Edit } from "../project";

/** Why a library file cannot be picked into this project, or `null` when it can. */
export function refusal(
  tile: LibraryTile,
  added: ReadonlySet<number>,
  t: Messages["assets"]["library"],
): string | null {
  return added.has(tile.id) ? t.inProject : null;
}

/** The edit a pick is: an import, which names no revision since it moves nothing. */
export function importEdit(item: number): Edit {
  return { tool: "import", args: { items: [item] }, edit: false };
}
