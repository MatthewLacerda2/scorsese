// The assets list's grouping: a sequence's stills under it, once each.

import { expect, test } from "bun:test";
import type { DocumentAsset } from "@/api";
import { grouped } from "./grouping";

const image = (id: string): DocumentAsset => ({ id, kind: "image", path: `assets/${id}.png` });
const sequence = (id: string, stills: string[]): DocumentAsset => ({
  id,
  kind: "image_sequence",
  sequence: { stills },
});
const shape = (list: ReturnType<typeof grouped>) =>
  list.map(({ asset, stills }) => [asset.id, stills.map((still) => still.id)]);

test("stills sit under their sequence in play order, once, and nowhere else", () => {
  const assets = [image("b"), image("a"), image("loose"), sequence("spin", ["a", "b", "a"])];
  expect(shape(grouped(assets))).toEqual([
    ["loose", []],
    ["spin", ["a", "b"]],
  ]);
});

test("a still two sequences play belongs to the first", () => {
  const assets = [image("a"), sequence("one", ["a"]), sequence("two", ["a"])];
  expect(shape(grouped(assets))).toEqual([
    ["one", ["a"]],
    ["two", []],
  ]);
});

test("a sequence naming something that is not an image hides nothing", () => {
  const assets = [{ id: "t", kind: "text" as const, text: "hi" }, sequence("spin", ["t"])];
  expect(shape(grouped(assets))).toEqual([
    ["t", []],
    ["spin", []],
  ]);
});
