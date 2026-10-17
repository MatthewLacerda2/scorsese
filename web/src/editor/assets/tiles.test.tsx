// The assets sidebar's tiles (#766): a file shows the library's thumbnail of
// its bytes, a brief nobody has made is hatched as its clip is, a sequence is
// one tile that unfolds to its stills, the corner bin sends `asset_remove`, and
// dragging a tile still places a clip. Rendered through `react-dom/server`, or
// taken apart as an element tree where a handler has to be called — no DOM.

import { describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { SCHEMA_VERSION } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { en } from "@/i18n/en";
import type { Edit, EditOutcome } from "../project";
import { assetRemoval, confirmThen } from "../removing";
import { AssetGrid } from "./AssetGrid";
import { handler, shot, tile } from "./handlers";
import { look } from "./look";

const sketch: DocumentAsset = { id: "dusk", kind: "generated_video", state: "sketch", prompt: "a" };
const still = (id: string): DocumentAsset => ({ id, kind: "image", sha256: id, path: `${id}.png` });
const sequence: DocumentAsset = {
  id: "Timelapse",
  kind: "image_sequence",
  sequence: { stills: ["s1", "s2"] },
};
const document = (assets: DocumentAsset[]): ProjectDocument => ({
  schema_version: SCHEMA_VERSION,
  name: "p",
  timeline_fps: { num: 30, den: 1 },
  assets,
  tracks: [],
});
const edit: EditOutcome = {
  run: async () => null,
  refused: null,
  dismiss: () => {},
  pending: false,
};
const libraryTile = (id: number) => ({
  id,
  name: "f",
  kind: "image" as const,
  size_bytes: 1,
  thumbnail: `/api/library/${id}/thumbnail`,
});

function grid(assets: DocumentAsset[], unfolded: string[] = []): string {
  const client = createQueryClient();
  client.setQueryData(["library", "list", { sha256: "aa" }], [libraryTile(7)]);
  client.setQueryData(["library", "list", { sha256: "s1" }], [libraryTile(11)]);
  client.setQueryData(["library", "list", { sha256: "s2" }], [libraryTile(12)]);
  return renderToString(
    <QueryClientProvider client={client}>
      <AssetGrid
        document={document(assets)}
        edit={edit}
        unfolded={new Set(unfolded)}
        onFold={() => {}}
        onOpen={() => {}}
      />
    </QueryClientProvider>,
  );
}

describe("an asset tile", () => {
  test("of a video shows its thumbnail, edged in its kind's colour", () => {
    const html = grid([shot]);
    expect(html).toContain('src="/api/library/7/thumbnail"');
    expect(html).toContain("bg-kind-video");
    expect(html).not.toContain("repeating-linear-gradient");
  });

  test("of a sketch is hatched, as its clip is, with no picture", () => {
    const html = grid([sketch]);
    expect(html).toContain("repeating-linear-gradient");
    expect(html).not.toContain("<img");
  });

  test("of a sequence is one tile showing its first still, unfolding to the stills", () => {
    const assets = [sequence, still("s1"), still("s2")];
    const folded = grid(assets);
    expect(folded).toContain("Timelapse — 2 photos");
    expect(folded.match(/<img/g)?.length).toBe(1);
    expect(folded).toContain("/api/library/11/thumbnail");
    expect(folded).not.toContain("/api/library/12/thumbnail");
    const open = grid(assets, ["Timelapse"]);
    expect(open.match(/<img/g)?.length).toBe(3);
    expect(open).toContain("/api/library/12/thumbnail");
    expect(open).toContain('aria-expanded="true"');
  });

  test("of a text, a page or a sound has a glyph, never a picture", () => {
    expect("picture" in look({ id: "t", kind: "text" })).toBe(false);
    expect("picture" in look({ id: "h", kind: "html", path: "p.html" })).toBe(false);
    expect("picture" in look({ id: "a", kind: "audio", sha256: "b", path: "a.wav" })).toBe(false);
  });

  test("removes through the corner bin: the confirm, then one asset_remove", async () => {
    const sent: Edit[] = [];
    const run = async (made: Edit) => void sent.push(made);
    const removal = assetRemoval(document([shot]), "beach", en.editor.removal);
    const view = tile(shot, () => confirmThen(removal, run, () => true));
    handler<() => void>(view, (props) => props["aria-label"] === "Remove beach", "onClick")();
    expect(sent.map((sentEdit) => sentEdit.tool)).toEqual(["asset_remove"]);
  });
});
