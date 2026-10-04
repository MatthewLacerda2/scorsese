// The library modal's picking rule: MIDI and files already in the project
// are refused, anything else is imported without being placed.

import { expect, test } from "bun:test";
import type { FileKind, LibraryTile } from "@/api";
import { en } from "@/i18n/en";
import { importEdit, PICKABLE_KINDS, refusal } from "./picking";

const tile = (id: number, kind: FileKind): LibraryTile => ({
  id,
  name: `file ${id}`,
  kind,
  size_bytes: 1,
  thumbnail: `/api/library/${id}/thumbnail`,
});

test("MIDI is never offered, since no track can hold it", () => {
  expect(PICKABLE_KINDS).toEqual(["video", "image", "audio"]);
  expect(refusal(tile(1, "midi"), new Set(), en.assets.library)).toBe("can't go on a track");
});

test("a file already in the project is refused; any other is picked", () => {
  const added = new Set([2]);
  expect(refusal(tile(2, "video"), added, en.assets.library)).toBe("in this project");
  expect(refusal(tile(3, "audio"), added, en.assets.library)).toBeNull();
});

test("a pick imports exactly that file and places nothing", () => {
  expect(importEdit(7)).toEqual({ tool: "import", args: { items: [7] }, edit: false });
});
