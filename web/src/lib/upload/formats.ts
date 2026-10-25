// What a tray takes: the extensions the library accepts, asked of the server
// (`GET /api/library/formats`) rather than copied, so a pick or a drop is
// refused by name exactly when an upload would be. And which drags are files:
// only one carrying `Files`, so an asset dragged onto the timeline is never
// mistaken for an attachment.

import type { FileKind, Formats } from "@/api";

/** The kind a file called `name` is, or `null` when the library takes no such file. */
export function kindOf(name: string, formats: Formats): FileKind | null {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return null;
  const extension = name.slice(dot + 1).toLowerCase();
  const kinds = Object.keys(formats) as FileKind[];
  return kinds.find((kind) => formats[kind].includes(extension)) ?? null;
}

/** A file a tray takes, and its kind — `null` while the list has not loaded. */
export interface Taken {
  file: File;
  kind: FileKind | null;
}

/**
 * `files` split into those a tray takes and the names of those it refuses.
 * Before the list has loaded nothing is refused: the upload is held to the
 * same list on the server, which then says so itself.
 */
export function sort(
  files: readonly File[],
  formats: Formats | undefined,
): { taken: Taken[]; refused: string[] } {
  const taken: Taken[] = [];
  const refused: string[] = [];
  for (const file of files) {
    const kind = formats ? kindOf(file.name, formats) : null;
    if (formats && kind === null) refused.push(file.name);
    else taken.push({ file, kind });
  }
  return { taken, refused };
}

/** The file picker's `accept` hint, or none while the list has not loaded. */
export function accept(formats: Formats | undefined): string | undefined {
  if (!formats) return undefined;
  return Object.values(formats)
    .flat()
    .map((extension) => `.${extension}`)
    .join(",");
}

/** Whether a drag is files from outside the page — readable during `dragover`. */
export function carriesFiles(types: readonly string[]): boolean {
  return types.includes("Files");
}
