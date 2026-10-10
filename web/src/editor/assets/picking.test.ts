// The library modal's picking rule: a file already in the project is
// refused, anything else is imported without being placed.

import { expect, test } from "bun:test";
import type { FileKind, LibraryTile } from "@/api";
import { en } from "@/i18n/en";
import { importEdit, refusal } from "./picking";

const tile = (id: number, kind: FileKind): LibraryTile => ({
  id,
  name: `file ${id}`,
  kind,
  size_bytes: 1,
  thumbnail: `/api/library/${id}/thumbnail`,
});

test("a file already in the project is refused; any other is picked", () => {
  const added = new Set([2]);
  expect(refusal(tile(2, "video"), added, en.assets.library)).toBe("in this project");
  expect(refusal(tile(3, "audio"), added, en.assets.library)).toBeNull();
});

test("a pick imports exactly that file and places nothing", () => {
  expect(importEdit(7)).toEqual({ tool: "import", args: { items: [7] }, edit: false });
});
