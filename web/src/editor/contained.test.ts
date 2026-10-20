// Every panel of the editor sits in an error boundary of its own (#1428), so a
// crash in one is contained there. That a panel boundary leaves its siblings
// drawn is `ErrorBoundary.test.tsx`'s; this holds the editor to using one
// around each panel, read from its source: making each real panel throw would
// mean replacing its module, which Bun does for the whole test run.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const PAGE = readFileSync(join(import.meta.dir, "EditorPage.tsx"), "utf8");

const PANELS = ["Inspector", "ChatPanel", "Preview", "AssetsPanel", "Timeline"];

test("each of the editor's panels has a boundary of its own", () => {
  const bare = PANELS.filter(
    (panel) => !new RegExp(`<ErrorBoundary place="panel">\\s*<${panel}[\\s/>]`).test(PAGE),
  );
  expect(bare).toEqual([]);
});

test("the panels are each drawn once, so none escapes its boundary", () => {
  for (const panel of PANELS) {
    expect({ panel, drawn: PAGE.split(`<${panel}`).length - 1 }).toEqual({ panel, drawn: 1 });
  }
});
