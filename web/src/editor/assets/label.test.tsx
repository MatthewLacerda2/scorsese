// What an asset tile says under its picture (#1006): its name, and nothing
// else unless there is money still to spend. The kind is the tile's colour and
// picture; how many clips show it was a bare "1×" nobody could read.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { DocumentAsset } from "@/api";
import { en } from "@/i18n/en";
import { TileView } from "./AssetTile";
import { look } from "./look";

function words(asset: DocumentAsset): string {
  const props = { asset, stills: 0, open: false, onFold: () => {}, pending: false };
  const view = { ...props, onRemove: () => {}, shown: look(asset), src: undefined };
  return renderToString(<TileView {...view} words={en.assets} />);
}

test("a made asset is its name only: no kind, no count", () => {
  const html = words({ id: "beach", kind: "video", sha256: "aa", path: "assets/aa.mp4" });
  expect(html).toContain(">beach</span>");
  expect(html).not.toContain(`>${en.assets.kinds.video}`);
  expect(html).not.toContain("×");
  expect(html).not.toContain("text-[11px]");
});

test("an unmade one also says its state, the money still to spend", () => {
  const html = words({ id: "dusk", kind: "generated_video", state: "sketch", prompt: "a" });
  expect(html).toContain(">dusk</span>");
  expect(html).toContain(`>${en.assets.state.sketch}</span>`);
  expect(html).not.toContain(en.assets.kinds.generated_video);
});
