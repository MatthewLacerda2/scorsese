// The motion kit (#812) for the landing page, joined from its pieces.
//
// A page's video loads the kit as one file, `https://lib.scorsese/kit.js`,
// which scorsese-render joins from `crates/render/src/page/shipped/kit/` in
// name order. The pieces are stretches of one script, not scripts of their
// own (only the first opens it, only the last closes it), kept apart so no
// source file outgrows the size gate (#1205). So the landing page cannot
// import them one by one: it imports `MOTION_KIT`, and this joins the pieces
// the same way — sorted by name, byte for byte — for `vite build` and the dev
// server (`motionKit`) and for `bun test` (`src/test/kit.ts`). A test in
// scorsese-render holds its own list of the pieces to that order.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";

/** What the landing page imports in place of the kit's file. */
export const MOTION_KIT = "virtual:motion-kit";

/** Where the kit's pieces are, in the repo and in the web image alike. */
export const KIT_DIR = fileURLToPath(
  new URL("../../../crates/render/src/page/shipped/kit/", import.meta.url),
);

/** Every piece of the kit, in the order it is joined. */
export function kitPieces(dir = KIT_DIR): string[] {
  return readdirSync(dir)
    .filter((name) => name.endsWith(".js"))
    .sort()
    .map((name) => join(dir, name));
}

/** The kit as a page's video gets it: every piece, joined. */
export function kitSource(dir = KIT_DIR): string {
  return kitPieces(dir)
    .map((piece) => readFileSync(piece, "utf8"))
    .join("");
}

/** Serves `MOTION_KIT` to Vite, re-reading the pieces whenever one changes. */
export function motionKit(): Plugin {
  const resolved = `\0${MOTION_KIT}`;
  return {
    name: "motion-kit",
    resolveId: (id) => (id === MOTION_KIT ? resolved : null),
    load(id) {
      if (id !== resolved) return null;
      for (const piece of kitPieces()) this.addWatchFile(piece);
      return kitSource();
    },
  };
}
