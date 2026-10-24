// A document in the library (kind `text`, a `.txt` or `.md`) is named as one,
// has no picture to wait for, and is never picked into a project as an asset.

import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { FILE_KINDS } from "@/api";
import { refusal } from "@/editor/assets/picking";
import { en } from "@/i18n/en";
import { Thumbnail } from "./Thumbnail";

test("a document is a kind the library filters by, with a name", () => {
  expect(FILE_KINDS).toContain("text");
  expect(en.files.kinds.text).toBe("Document");
});

test("a document's tile shows the kind's icon and never asks for a thumbnail", () => {
  const html = renderToStaticMarkup(<Thumbnail src="/api/library/9/thumbnail" kind="text" />);
  expect(html).not.toContain("<img");
  expect(html).toContain("<svg");
});

test("a document is refused by the project's picker: the assistant reads it", () => {
  const tile = { id: 4, name: "script.md", kind: "text", size_bytes: 1, thumbnail: "" } as const;
  expect(refusal(tile, new Set(), en.assets.library)).toBe("for the assistant to read");
});
