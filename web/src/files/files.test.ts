// The file browser's logic: ordering, sizes and durations, and reading the
// refusal that keeps a used file from being deleted.

import { expect, test } from "bun:test";
import { ApiError, type LibraryTile } from "@/api";
import { formatBytes, formatDuration } from "@/lib/format";
import { safeNext } from "@/session/session";
import { inUseBy } from "./DetailsSheet";
import { sortTiles } from "./sort";

const tile = (id: number, name: string, size_bytes: number): LibraryTile => ({
  id,
  name,
  kind: "video",
  size_bytes,
  thumbnail: `/api/library/${id}/thumbnail`,
});

test("files sort by arrival, name (numbers in order) or size", () => {
  const tiles = [tile(1, "clip 10", 50), tile(3, "Clip 2", 10), tile(2, "b-roll", 900)];
  const ids = (order: Parameters<typeof sortTiles>[1]) => sortTiles(tiles, order).map((t) => t.id);
  expect(ids("newest")).toEqual([3, 2, 1]);
  expect(ids("oldest")).toEqual([1, 2, 3]);
  expect(ids("name")).toEqual([2, 3, 1]);
  expect(ids("largest")).toEqual([2, 1, 3]);
  expect(tiles.map((t) => t.id)).toEqual([1, 3, 2]);
});

test("sizes and durations read as a file manager shows them", () => {
  expect(formatBytes(999)).toBe("999 B");
  expect(formatBytes(1_536_000)).toBe("1.5 MB");
  expect(formatBytes(250_000_000)).toBe("250 MB");
  expect(formatDuration(75.4)).toBe("1:15");
  expect(formatDuration(3723)).toBe("1:02:03");
});

test("a 409 naming projects is read as the projects keeping a file", () => {
  const projects = [{ id: 2, name: "teaser" }];
  expect(inUseBy(new ApiError(409, "in use", { projects }))).toEqual(projects);
  expect(inUseBy(new ApiError(409, "moved on"))).toBeNull();
  expect(inUseBy(new ApiError(404, "gone", { projects }))).toBeNull();
  expect(inUseBy(new Error("x"))).toBeNull();
});

test("after logging in, only a path on this site is followed", () => {
  expect(safeNext("/library?item=3")).toBe("/library?item=3");
  expect(safeNext(null)).toBe("/");
  expect(safeNext("https://evil.example")).toBe("/");
  expect(safeNext("//evil.example")).toBe("/");
  expect(safeNext("/\\evil.example")).toBe("/");
});
