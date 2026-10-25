// An attachment's state: held in memory signed out, else its upload row's.

import { expect, test } from "bun:test";
import type { Upload } from "@/files/uploads";
import { busy, isSettled, items, statusOf } from "./status";

const row = (change: Partial<Upload>): Upload => ({
  key: "upload-1",
  name: "a.png",
  phase: "hashing",
  progress: 0,
  ...change,
});

test("signed out, a file is only held", () => {
  expect(statusOf(undefined, [])).toEqual({ phase: "held", progress: 1 });
});

test("hashing and uploading are one climb, half each", () => {
  expect(statusOf("upload-1", [row({ progress: 0.5 })])).toEqual({
    phase: "uploading",
    progress: 0.25,
  });
  expect(statusOf("upload-1", [row({ phase: "uploading", progress: 0.5 })])).toEqual({
    phase: "uploading",
    progress: 0.75,
  });
});

test("a file the library already has is ready, as its item: it never re-crossed the network", () => {
  expect(statusOf("upload-1", [row({ phase: "duplicate", item: 7 })])).toEqual({
    phase: "ready",
    progress: 1,
    item: 7,
  });
  expect(statusOf("upload-1", [row({ phase: "done", item: 8 })])).toMatchObject({ item: 8 });
});

test("a refusal carries the server's words", () => {
  expect(statusOf("upload-1", [row({ phase: "failed", message: "not a video" })])).toEqual({
    phase: "failed",
    progress: 0,
    message: "not a video",
  });
});

test("sending waits while any is on its way, and sends the items that are ready", () => {
  const statuses = [
    statusOf("upload-1", [row({ phase: "done", item: 3 })]),
    statusOf(undefined, []),
  ];
  expect(busy(statuses)).toBe(false);
  expect(items(statuses)).toEqual([3]);
  expect(busy([...statuses, statusOf("upload-1", [row({})])])).toBe(true);
});

test("a settled file stays settled once its row is cleared from the uploads corner", () => {
  const ready = statusOf("upload-1", [row({ phase: "done", item: 3 })]);
  expect(isSettled(ready)).toBe(true);
  expect(statusOf("upload-1", [], ready)).toEqual(ready);
  expect(isSettled(statusOf("upload-1", [row({})]))).toBe(false);
});
