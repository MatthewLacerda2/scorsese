// The kit the landing page gets is the one a video's page gets.

import { expect, test } from "bun:test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { kitPieces, kitSource } from "@/build/kit";

test("the pieces are joined in name order, whatever order the disk lists them", () => {
  const dir = mkdtempSync(join(tmpdir(), "kit-"));
  writeFileSync(join(dir, "02-b.js"), "b;\n");
  writeFileSync(join(dir, "01-a.js"), "(a;\n");
  writeFileSync(join(dir, "README.md"), "not a piece");
  expect(kitPieces(dir).map((piece) => piece.slice(dir.length + 1))).toEqual([
    "01-a.js",
    "02-b.js",
  ]);
  expect(kitSource(dir)).toBe("(a;\nb;\n");
});

test("the joined kit is one whole script defining `window.kit`", () => {
  const kit = kitSource();
  expect(kit.startsWith("// scorsese's motion kit")).toBe(true);
  expect(kit.endsWith("})();\n")).toBe(true);
  expect(kit).toContain("window.kit = Object.freeze({");
});
