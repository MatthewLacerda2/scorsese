// What the editor's library modal may pick, and what a pick does (#702). A
// pick brings the file into the project's assets (`import`, which never adds
// a file twice) without placing it: the user drags it onto a lane from there.

import { FILE_KINDS, type FileKind, type LibraryTile } from "@/api";
import type { Messages } from "@/i18n/catalogue";
import type { Edit } from "../project";

/**
 * The kinds a project can hold. MIDI is not one: it is notes, not media a
 * track can hold, and the assistant reads it into a song (#678).
 */
export const PICKABLE_KINDS: readonly FileKind[] = FILE_KINDS.filter((kind) => kind !== "midi");

/** Why a library file cannot be picked into this project, or `null` when it can. */
export function refusal(
  tile: LibraryTile,
  added: ReadonlySet<number>,
  t: Messages["assets"]["library"],
): string | null {
  if (!PICKABLE_KINDS.includes(tile.kind)) return t.notTrack;
  if (added.has(tile.id)) return t.inProject;
  return null;
}

/** The edit a pick is: an import, which names no revision since it moves nothing. */
export function importEdit(item: number): Edit {
  return { tool: "import", args: { items: [item] }, edit: false };
}
