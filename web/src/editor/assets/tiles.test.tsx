// The assets sidebar's tiles (#766): a file shows the library's thumbnail of
// its bytes, a brief nobody has made is hatched as its clip is, a sequence is
// one tile that unfolds to its stills, the corner bin sends `asset_remove`, and
// dragging a tile still places a clip. Rendered through `react-dom/server`, or
// taken apart as an element tree where a handler has to be called — no DOM.

import { describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import type { DragEvent, ReactElement, ReactNode } from "react";
import { renderToString } from "react-dom/server";
import type { DocumentAsset, EditorProject, ProjectDocument, ToolAnswer } from "@/api";
import { createQueryClient } from "@/app/queryClient";
import { en } from "@/i18n/en";
import { dropOnto } from "../drop";
import type { Edit, EditOutcome } from "../project";
import { assetRemoval, confirmThen } from "../removing";
import { AssetGrid } from "./AssetGrid";
import { TileView } from "./AssetTile";
import { dropped } from "./dragged";
import { look } from "./look";

const shot: DocumentAsset = { id: "beach", kind: "video", sha256: "aa", path: "assets/aa.mp4" };
const sketch: DocumentAsset = { id: "dusk", kind: "generated_video", state: "sketch", prompt: "a" };
const still = (id: string): DocumentAsset => ({ id, kind: "image", sha256: id, path: `${id}.png` });
const sequence: DocumentAsset = {
  id: "Timelapse",
  kind: "image_sequence",
  sequence: { stills: ["s1", "s2"] },
};
const document = (assets: DocumentAsset[]): ProjectDocument => ({
  schema_version: 1,
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
      />
    </QueryClientProvider>,
  );
}

/** Every element in a tree, depth first — to find a handler and call it. */
function* elements(node: ReactNode): Generator<ReactElement<Record<string, unknown>>> {
  if (Array.isArray(node)) for (const child of node) yield* elements(child);
  else if (node && typeof node === "object" && "props" in node) {
    const element = node as ReactElement<Record<string, unknown>>;
    yield element;
    yield* elements(element.props.children as ReactNode);
  }
}

/** The handler `key` of the first element `match` picks — failing loudly when none does. */
function handler<F>(
  view: ReactNode,
  match: (props: Record<string, unknown>) => boolean,
  key: string,
) {
  const found = [...elements(view)].find((element) => match(element.props));
  if (!found) throw new Error(`no element has ${key}`);
  return found.props[key] as F;
}

function tile(asset: DocumentAsset, onRemove = () => {}) {
  const props = { asset, stills: 0, open: false, onFold: () => {}, pending: false };
  return TileView({ ...props, onRemove, shown: look(asset), src: undefined, words: en.assets });
}

describe("an asset tile", () => {
  test("of a video shows its thumbnail, edged in its kind's colour", () => {
    const html = grid([shot]);
    expect(html).toContain('src="/api/library/7/thumbnail"');
    expect(html).toContain("bg-sky-500");
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
    const run = async (edit: Edit) => void sent.push(edit);
    const removal = assetRemoval(document([shot]), "beach", en.editor.removal);
    const view = tile(shot, () => confirmThen(removal, run, () => true));
    handler<() => void>(view, (props) => props["aria-label"] === "Remove beach", "onClick")();
    expect(sent.map((edit) => edit.tool)).toEqual(["asset_remove"]);
  });

  test("dragged onto the timeline still places a clip", async () => {
    const data = new Map<string, string>();
    const transfer = {
      setData: (k: string, v: string) => data.set(k, v),
      getData: (k: string) => data.get(k) ?? "",
    };
    const event = { dataTransfer: transfer } as unknown as DragEvent;
    handler<(event: DragEvent) => void>(
      tile(shot),
      (props) => props.draggable === true,
      "onDragStart",
    )(event);
    const carried = dropped(event);
    expect(carried).toEqual({ from: "project", asset: "beach", kind: "video" });

    const calls: string[] = [];
    const before = { revision: 1, document: document([shot]) } as unknown as EditorProject;
    const run = async (call: Edit): Promise<ToolAnswer | null> => {
      calls.push(call.tool);
      const doc = structuredClone(before.document);
      doc.tracks = [{ id: "v1", kind: "video", clips: [] }];
      if (call.tool === "place_clip")
        doc.tracks[0]?.clips.push({ id: "c1", asset: "beach", start: 0, duration: 30 });
      return { said: [], project: { ...before, document: doc } };
    };
    expect(await dropOnto(run, before, carried as NonNullable<typeof carried>, null, 0, 0, 0)).toBe(
      "c1",
    );
    expect(calls).toEqual(["track_new", "place_clip"]);
  });
});
